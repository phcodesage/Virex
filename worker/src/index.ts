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

/**
 * Licences live in KV as `license:<key>` = "active", written by the Ko-fi
 * webhook. Monthly subscriptions get a 35-day TTL that each payment refreshes,
 * so a cancelled subscription lapses on its own — Ko-fi sends no "cancelled"
 * event we could rely on.
 */
async function isPro(env: Env, licence: string | null): Promise<boolean> {
  if (!licence) return false;
  return (await env.VIREX_KV.get(`license:${licence}`)) === "active";
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
  const pro = await isPro(env, licenceOf(request));
  const limit = Number(env.FREE_DAILY_LIMIT ?? "10");
  const used = Number((await env.VIREX_KV.get(`usage:${device}:${today()}`)) ?? "0");
  return {
    pro,
    limit,
    used,
    remaining: pro ? Number.POSITIVE_INFINITY : Math.max(0, limit - used),
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
    return json(
      {
        error: "daily_limit_reached",
        limit: plan.limit,
        used: plan.used,
        message: `You've used all ${plan.limit} free rewrites today. Upgrade to Pro for unlimited.`,
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
  const body = [
    `<p>Thanks for supporting Virex!</p>`,
    `<p>Your Pro licence key is:</p>`,
    `<p style="font-family:monospace;font-size:18px"><strong>${key}</strong></p>`,
    `<p>Open Virex &rarr; Settings &rarr; <em>I have a key</em>, paste it in, and press Activate.</p>`,
    `<p>It stays active as long as your Ko-fi subscription is running.</p>`,
  ].join("");

  try {
    const resp = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${env.RESEND_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ from: env.LICENSE_FROM_EMAIL, to, subject, html: body }),
    });
    return resp.ok;
  } catch {
    return false;
  }
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
  return json({ found: true, key, active });
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
