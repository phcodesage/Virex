/**
 * Virex API proxy.
 *
 * Holds the DeepSeek key server-side so it never ships inside the macOS app,
 * and enforces the plan limits (free: N rewrites/day, Pro: unlimited) where a
 * client can't tamper with them.
 *
 * Routes:
 *   POST /v1/rewrite  — stream a rewrite (SSE passthrough from DeepSeek)
 *   GET  /v1/usage    — today's usage for a device, for the Settings window
 */

export interface Env {
  /** `wrangler secret put DEEPSEEK_API_KEY` */
  DEEPSEEK_API_KEY: string;
  /** KV holding daily counters and Pro licences. */
  VIREX_KV: KVNamespace;
  /** Optional override; defaults to 10. */
  FREE_DAILY_LIMIT?: string;
  /** Upstream model id; defaults to deepseek-v4-flash. */
  MODEL?: string;
  /** Devices one licence may be used on; defaults to 3. */
  MAX_SEATS?: string;
  /** From Ko-fi's webhook settings — proves a webhook really came from Ko-fi. */
  KOFI_VERIFICATION_TOKEN?: string;
  /** Minimum payment (in Ko-fi's currency units) that grants Pro. Default 10. */
  PRO_MIN_AMOUNT?: string;
  /** Optional: set to have the Worker email the licence key itself. */
  RESEND_API_KEY?: string;
  /** From-address for that email, e.g. "Virex <keys@yourdomain>". */
  LICENSE_FROM_EMAIL?: string;
  /** Shared secret guarding the admin lookup endpoint. */
  ADMIN_TOKEN?: string;
}

const DEEPSEEK_URL = "https://api.deepseek.com/chat/completions";
/** DeepSeek retired the old `deepseek-chat` id; current names are v4-pro/v4-flash. */
const DEFAULT_MODEL = "deepseek-v4-flash";
/** Refuse absurd inputs so one device can't burn the budget in a few calls. */
const MAX_INPUT_CHARS = 6000;
/** Counters only need to outlive the day they describe. */
const COUNTER_TTL_SECONDS = 60 * 60 * 48;

const DEFAULT_SYSTEM_PROMPT = `You are an expert writing assistant that rewrites and paraphrases the user's text.

Rules:
- Rewrite the text to be clear, natural, fluent, and grammatically correct.
- Actively rephrase awkward or unnatural wording rather than copying it.
- Fix spelling, grammar, and punctuation.
- When the text is contradictory or ambiguous, infer the single most likely intended meaning and commit to it in one clean sentence. Do not hedge with "and/or" or list both options.
- Preserve the user's core intent, tone, and language.
- Preserve links, markdown, emojis, and formatting.
- Never explain your changes.
- Return ONLY the rewritten text.`;

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "OPTIONS") return cors(new Response(null, { status: 204 }));

    try {
      if (url.pathname === "/v1/usage" && request.method === "GET") {
        return cors(await handleUsage(request, env));
      }
      if (url.pathname === "/v1/rewrite" && request.method === "POST") {
        return cors(await handleRewrite(request, env));
      }
      if (url.pathname === "/webhooks/kofi" && request.method === "POST") {
        return await handleKofiWebhook(request, env);
      }
      if (url.pathname === "/admin/license" && request.method === "GET") {
        return await handleAdminLookup(request, env, url);
      }
      if (url.pathname === "/admin/reset-seats" && request.method === "POST") {
        return await handleResetSeats(request, env, url);
      }
      if (url.pathname === "/") {
        return cors(json({ ok: true, service: "virex-api" }));
      }
      return cors(json({ error: "not_found" }, 404));
    } catch (err) {
      return cors(json({ error: "internal", message: String(err) }, 500));
    }
  },
};

/* ------------------------------------------------------------------ plans */

interface Plan {
  pro: boolean;
  limit: number;
  used: number;
  remaining: number;
  seatLimited: boolean;
  seatsUsed: number;
  maxSeats: number;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function deviceOf(request: Request): string | null {
  const id = request.headers.get("X-Virex-Device")?.trim();
  // Cheap sanity check: our client sends a UUID.
  if (!id || id.length < 8 || id.length > 100) return null;
  return id;
}

function licenceOf(request: Request): string | null {
  const auth = request.headers.get("Authorization") ?? "";
  const m = auth.match(/^Bearer\s+(.+)$/i);
  return m ? m[1].trim() : null;
}

const DEFAULT_MAX_SEATS = 3;

export interface Entitlement {
  pro: boolean;
  /** Licence is valid, but this device couldn't claim a seat. */
  seatLimited: boolean;
  seatsUsed: number;
  maxSeats: number;
}

/**
 * Whether this licence entitles *this device* to Pro.
 *
 * Licences live in KV as `license:<key>` = "active", written by the Ko-fi
 * webhook, with a 35-day TTL that each payment refreshes — so a cancelled
 * subscription lapses on its own (Ko-fi sends no reliable "cancelled" event).
 *
 * A licence also claims seats: the first `MAX_SEATS` devices to use it are
 * remembered, and any device beyond that is refused. That's what stops one key
 * being passed around a group chat.
 */
async function entitlementFor(
  env: Env,
  licence: string | null,
  device: string,
): Promise<Entitlement> {
  const maxSeats = Math.max(1, Number(env.MAX_SEATS ?? DEFAULT_MAX_SEATS));
  const none: Entitlement = { pro: false, seatLimited: false, seatsUsed: 0, maxSeats };

  if (!licence) return none;
  if ((await env.VIREX_KV.get(`license:${licence}`)) !== "active") return none;

  let devices: string[] = [];
  try {
    const raw = await env.VIREX_KV.get(`seats:${licence}`);
    if (raw) devices = JSON.parse(raw);
    if (!Array.isArray(devices)) devices = [];
  } catch {
    devices = [];
  }

  // Already a known device — always allowed, and doesn't consume a new seat.
  if (devices.includes(device)) {
    return { pro: true, seatLimited: false, seatsUsed: devices.length, maxSeats };
  }

  if (devices.length < maxSeats) {
    devices.push(device);
    await env.VIREX_KV.put(`seats:${licence}`, JSON.stringify(devices), {
      expirationTtl: LICENSE_TTL_SECONDS,
    });
    return { pro: true, seatLimited: false, seatsUsed: devices.length, maxSeats };
  }

  return { pro: false, seatLimited: true, seatsUsed: devices.length, maxSeats };
}

/** A month plus a few days' grace, so a slightly late renewal doesn't lock someone out. */
const LICENSE_TTL_SECONDS = 60 * 60 * 24 * 35;

function newLicenseKey(): string {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no look-alikes
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  const chars = [...bytes].map((b) => alphabet[b % alphabet.length]);
  return `VIREX-${chars.slice(0, 4).join("")}-${chars.slice(4).join("")}`;
}

async function planFor(request: Request, env: Env, device: string): Promise<Plan> {
  const ent = await entitlementFor(env, licenceOf(request), device);
  const limit = Number(env.FREE_DAILY_LIMIT ?? "10");
  const used = Number((await env.VIREX_KV.get(`usage:${device}:${today()}`)) ?? "0");
  return {
    pro: ent.pro,
    limit,
    used,
    remaining: ent.pro ? Number.POSITIVE_INFINITY : Math.max(0, limit - used),
    seatLimited: ent.seatLimited,
    seatsUsed: ent.seatsUsed,
    maxSeats: ent.maxSeats,
  };
}

async function bumpUsage(env: Env, device: string, used: number): Promise<void> {
  await env.VIREX_KV.put(`usage:${device}:${today()}`, String(used + 1), {
    expirationTtl: COUNTER_TTL_SECONDS,
  });
}

/* --------------------------------------------------------------- handlers */

async function handleUsage(request: Request, env: Env): Promise<Response> {
  const device = deviceOf(request);
  if (!device) return json({ error: "missing_device" }, 400);

  const plan = await planFor(request, env, device);
  return json({
    plan: plan.pro ? "pro" : "free",
    used: plan.used,
    limit: plan.pro ? null : plan.limit,
    remaining: plan.pro ? null : plan.remaining,
    seat_limited: plan.seatLimited,
    seats_used: plan.seatsUsed,
    max_seats: plan.maxSeats,
  });
}

async function handleRewrite(request: Request, env: Env): Promise<Response> {
  const device = deviceOf(request);
  if (!device) return json({ error: "missing_device" }, 400);

  const body = (await request.json().catch(() => null)) as { text?: string; system?: string } | null;
  const text = body?.text?.trim();
  if (!text) return json({ error: "missing_text" }, 400);
  if (text.length > MAX_INPUT_CHARS) return json({ error: "text_too_long", max: MAX_INPUT_CHARS }, 413);

  const plan = await planFor(request, env, device);
  if (!plan.pro && plan.remaining <= 0) {
    const message = plan.seatLimited
      ? `This licence is already in use on ${plan.maxSeats} devices, so this one is on the free plan — and today's ${plan.limit} free rewrites are gone.`
      : `You've used all ${plan.limit} free rewrites today. Upgrade to Pro for unlimited.`;
    return json(
      {
        error: plan.seatLimited ? "seat_limit_reached" : "daily_limit_reached",
        limit: plan.limit,
        used: plan.used,
        seat_limited: plan.seatLimited,
        message,
      },
      429,
    );
  }

  const upstream = await fetch(DEEPSEEK_URL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.DEEPSEEK_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: env.MODEL?.trim() || DEFAULT_MODEL,
      temperature: 0.2,
      stream: true,
      messages: [
        { role: "system", content: body?.system?.trim() || DEFAULT_SYSTEM_PROMPT },
        { role: "user", content: text },
      ],
    }),
  });

  if (!upstream.ok || !upstream.body) {
    const detail = await upstream.text().catch(() => "");
    return json({ error: "upstream_error", status: upstream.status, detail: detail.slice(0, 300) }, 502);
  }

  // Only bill the request once upstream has actually accepted it.
  await bumpUsage(env, device, plan.used);

  return new Response(upstream.body, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream; charset=utf-8",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
      "X-Virex-Plan": plan.pro ? "pro" : "free",
      "X-Virex-Remaining": plan.pro ? "unlimited" : String(Math.max(0, plan.remaining - 1)),
    },
  });
}

/* ---------------------------------------------------------- ko-fi webhook */

interface KofiPayload {
  verification_token?: string;
  type?: string;
  email?: string;
  amount?: string;
  from_name?: string;
  is_subscription_payment?: boolean;
  is_first_subscription_payment?: boolean;
  kofi_transaction_id?: string;
  tier_name?: string;
}

/**
 * Ko-fi posts `data=<json>` as form-encoded on every payment. A qualifying
 * payment issues (or renews) a licence key for that email address.
 */
async function handleKofiWebhook(request: Request, env: Env): Promise<Response> {
  const form = await request.formData().catch(() => null);
  const raw = form?.get("data");
  if (typeof raw !== "string") return json({ error: "bad_payload" }, 400);

  let payload: KofiPayload;
  try {
    payload = JSON.parse(raw);
  } catch {
    return json({ error: "bad_json" }, 400);
  }

  // Ko-fi's shared token is the only thing proving this is really from Ko-fi.
  const expected = env.KOFI_VERIFICATION_TOKEN;
  if (!expected || payload.verification_token !== expected) {
    return json({ error: "bad_token" }, 401);
  }

  const email = payload.email?.trim().toLowerCase();
  if (!email) return json({ ok: true, note: "no email on payment; nothing to issue" });

  const amount = Number(payload.amount ?? "0");
  const minimum = Number(env.PRO_MIN_AMOUNT ?? "10");
  if (!Number.isFinite(amount) || amount < minimum) {
    // A tip, not a Pro subscription — thank them and move on.
    return json({ ok: true, note: "below Pro threshold; treated as a tip" });
  }

  // Reuse the same key across renewals so the customer never has to re-paste it.
  const existing = await env.VIREX_KV.get(`kofi:${email}`);
  const key = existing ?? newLicenseKey();

  await env.VIREX_KV.put(`license:${key}`, "active", { expirationTtl: LICENSE_TTL_SECONDS });
  // Keep the email→key map alive longer than the licence so renewals find it.
  await env.VIREX_KV.put(`kofi:${email}`, key, { expirationTtl: LICENSE_TTL_SECONDS * 6 });

  const emailed = await sendLicenseEmail(env, email, key, Boolean(existing));

  return json({ ok: true, issued: !existing, emailed });
}

/**
 * Email the key via Resend, when configured. Returns whether it was sent — if
 * not, the key is still stored and can be looked up with /admin/license.
 */
async function sendLicenseEmail(
  env: Env,
  to: string,
  key: string,
  isRenewal: boolean,
): Promise<boolean> {
  if (!env.RESEND_API_KEY || !env.LICENSE_FROM_EMAIL) return false;

  const subject = isRenewal ? "Your Virex Pro licence (renewed)" : "Your Virex Pro licence key";

  try {
    const resp = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${env.RESEND_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        from: env.LICENSE_FROM_EMAIL,
        to,
        subject,
        html: licenseEmailHtml(key, isRenewal),
        text: licenseEmailText(key, isRenewal),
      }),
    });
    return resp.ok;
  } catch {
    return false;
  }
}

/**
 * Licence email. Table-based with inline styles — email clients strip <style>
 * blocks and don't support flex/grid.
 */
function licenseEmailHtml(key: string, isRenewal: boolean): string {
  const heading = isRenewal ? "Your Pro licence is renewed" : "Welcome to Virex Pro";
  const lede = isRenewal
    ? "Thanks for sticking around. Your subscription renewed — the key below is the same one you already have, no need to re-enter it."
    : "Thanks for supporting Virex. Unlimited rewrites are ready to switch on.";

  return `<!doctype html>
<html>
<body style="margin:0;padding:0;background:#f4f6fb;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">${heading} — your key is ${key}</div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background:#f4f6fb;padding:32px 12px;">
    <tr><td align="center">
      <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:520px;background:#ffffff;border-radius:16px;overflow:hidden;box-shadow:0 2px 8px rgba(16,24,40,0.06);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Inter,Helvetica,Arial,sans-serif;">

        <tr><td style="background:#3580fb;background-image:linear-gradient(135deg,#3580fb,#1a57d9);padding:32px 32px 28px;">
          <table role="presentation" cellpadding="0" cellspacing="0"><tr>
            <td style="padding-right:12px;">
              <img src="https://virex-eta.vercel.app/icon.png" width="40" height="40" alt="" style="display:block;border-radius:10px;" />
            </td>
            <td style="font-size:20px;font-weight:700;color:#ffffff;letter-spacing:-0.3px;">Virex</td>
          </tr></table>
          <div style="margin-top:20px;font-size:24px;line-height:1.25;font-weight:700;color:#ffffff;letter-spacing:-0.5px;">${heading}</div>
        </td></tr>

        <tr><td style="padding:28px 32px 8px;font-size:15px;line-height:1.6;color:#475467;">${lede}</td></tr>

        <tr><td style="padding:16px 32px 4px;">
          <div style="font-size:11px;font-weight:700;letter-spacing:1.2px;text-transform:uppercase;color:#98a2b3;padding-bottom:8px;">Your licence key</div>
          <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
            <tr><td align="center" style="background:#f2f6ff;border:1px solid #d6e2ff;border-radius:10px;padding:18px 12px;font-family:'SF Mono',Menlo,Consolas,monospace;font-size:21px;font-weight:700;color:#1a57d9;letter-spacing:1.5px;">${key}</td></tr>
          </table>
        </td></tr>

        <tr><td style="padding:26px 32px 4px;">
          <div style="font-size:11px;font-weight:700;letter-spacing:1.2px;text-transform:uppercase;color:#98a2b3;padding-bottom:12px;">Activate it</div>
          <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="font-size:14px;line-height:1.5;color:#475467;">
            <tr><td width="26" valign="top" style="padding-bottom:10px;color:#3580fb;font-weight:700;">1</td><td style="padding-bottom:10px;">Open Virex from your menu bar and choose <strong style="color:#101828;">Open Settings</strong></td></tr>
            <tr><td width="26" valign="top" style="padding-bottom:10px;color:#3580fb;font-weight:700;">2</td><td style="padding-bottom:10px;">Click <strong style="color:#101828;">I have a key</strong></td></tr>
            <tr><td width="26" valign="top" color="#3580fb" style="color:#3580fb;font-weight:700;">3</td><td>Paste the key and press <strong style="color:#101828;">Activate</strong></td></tr>
          </table>
        </td></tr>

        <tr><td style="padding:24px 32px 30px;">
          <table role="presentation" width="100%" cellpadding="0" cellspacing="0"><tr>
            <td align="center" style="background:#3580fb;border-radius:10px;">
              <a href="https://github.com/phcodesage/Virex/releases/latest" style="display:block;padding:13px 24px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;">Download the latest Virex</a>
            </td>
          </tr></table>
        </td></tr>

        <tr><td style="padding:20px 32px 28px;border-top:1px solid #eaecf0;font-size:12.5px;line-height:1.6;color:#98a2b3;">
          Your key stays active while your Ko-fi subscription is running.
          Lost it? Just reply to this email.
          <div style="padding-top:10px;">— Virex · <a href="https://virex-eta.vercel.app" style="color:#3580fb;text-decoration:none;">virex-eta.vercel.app</a></div>
        </td></tr>

      </table>
    </td></tr>
  </table>
</body>
</html>`;
}

/** Plain-text alternative — improves deliverability and covers text-only clients. */
function licenseEmailText(key: string, isRenewal: boolean): string {
  return [
    isRenewal ? "Your Virex Pro licence is renewed" : "Welcome to Virex Pro",
    "",
    isRenewal
      ? "Your subscription renewed. This is the same key you already have."
      : "Thanks for supporting Virex. Unlimited rewrites are ready to switch on.",
    "",
    `Your licence key: ${key}`,
    "",
    "Activate it:",
    "  1. Open Virex from your menu bar and choose Open Settings",
    "  2. Click 'I have a key'",
    "  3. Paste the key and press Activate",
    "",
    "Download the latest Virex: https://github.com/phcodesage/Virex/releases/latest",
    "",
    "Your key stays active while your Ko-fi subscription is running.",
    "Lost it? Just reply to this email.",
    "",
    "— Virex · https://virex-eta.vercel.app",
  ].join("\n");
}

/**
 * Clear a licence's claimed devices — for the "I bought a new Mac" support case.
 * The next `MAX_SEATS` devices to use the key claim the freed seats.
 */
async function handleResetSeats(request: Request, env: Env, url: URL): Promise<Response> {
  const token = request.headers.get("X-Admin-Token");
  if (!env.ADMIN_TOKEN || token !== env.ADMIN_TOKEN) return json({ error: "unauthorized" }, 401);

  const key = url.searchParams.get("key")?.trim();
  if (!key) return json({ error: "missing_key" }, 400);

  await env.VIREX_KV.delete(`seats:${key}`);
  return json({ ok: true, key, seats_cleared: true });
}

/** Look up the key issued to an email, for answering "where's my key?" support. */
async function handleAdminLookup(request: Request, env: Env, url: URL): Promise<Response> {
  const token = request.headers.get("X-Admin-Token");
  if (!env.ADMIN_TOKEN || token !== env.ADMIN_TOKEN) return json({ error: "unauthorized" }, 401);

  const email = url.searchParams.get("email")?.trim().toLowerCase();
  if (!email) return json({ error: "missing_email" }, 400);

  const key = await env.VIREX_KV.get(`kofi:${email}`);
  if (!key) return json({ found: false });

  const active = (await env.VIREX_KV.get(`license:${key}`)) === "active";
  let seats: string[] = [];
  try {
    const raw = await env.VIREX_KV.get(`seats:${key}`);
    if (raw) seats = JSON.parse(raw);
    if (!Array.isArray(seats)) seats = [];
  } catch {
    seats = [];
  }
  return json({
    found: true,
    key,
    active,
    seats_used: seats.length,
    max_seats: Math.max(1, Number(env.MAX_SEATS ?? DEFAULT_MAX_SEATS)),
  });
}

/* ----------------------------------------------------------------- helpers */

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json; charset=utf-8" },
  });
}

function cors(res: Response): Response {
  const h = new Headers(res.headers);
  h.set("Access-Control-Allow-Origin", "*");
  h.set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Virex-Device");
  h.set("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  return new Response(res.body, { status: res.status, headers: h });
}
