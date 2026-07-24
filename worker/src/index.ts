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
  /** Lemon Squeezy store/product ids, so keys from other stores are rejected. */
  LEMONSQUEEZY_STORE_ID?: string;
  LEMONSQUEEZY_PRODUCT_ID?: string;
}

const DEEPSEEK_URL = "https://api.deepseek.com/chat/completions";
const MODEL = "deepseek-chat";
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

/** Cache a positive result for 12h and a negative one for 1h, so a cancellation
 *  takes effect within half a day and a fresh purchase works within the hour. */
const PRO_CACHE_OK_TTL = 60 * 60 * 12;
const PRO_CACHE_BAD_TTL = 60 * 60;

/**
 * Is this licence currently entitled to Pro?
 *
 * Checks, in order: a manually-issued key in KV (for comps and refunds), a
 * cached verdict, then Lemon Squeezy itself. Caching keeps the hot rewrite path
 * from making a third-party call on every request.
 */
async function isPro(env: Env, licence: string | null): Promise<boolean> {
  if (!licence) return false;

  // Manually issued / comped keys.
  if ((await env.VIREX_KV.get(`license:${licence}`)) === "active") return true;

  const cached = await env.VIREX_KV.get(`pro:${licence}`);
  if (cached === "active") return true;
  if (cached === "inactive") return false;

  const ok = await validateWithLemonSqueezy(env, licence);
  await env.VIREX_KV.put(`pro:${licence}`, ok ? "active" : "inactive", {
    expirationTtl: ok ? PRO_CACHE_OK_TTL : PRO_CACHE_BAD_TTL,
  });
  return ok;
}

/** Ask Lemon Squeezy whether a licence key is active for our product. */
async function validateWithLemonSqueezy(env: Env, licence: string): Promise<boolean> {
  try {
    const resp = await fetch("https://api.lemonsqueezy.com/v1/licenses/validate", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({ license_key: licence }),
    });
    if (!resp.ok) return false;

    const data = (await resp.json()) as {
      valid?: boolean;
      license_key?: { status?: string };
      meta?: { store_id?: number; product_id?: number };
    };
    if (!data.valid || data.license_key?.status !== "active") return false;

    // Reject otherwise-valid keys issued by a different store or product.
    const wantStore = env.LEMONSQUEEZY_STORE_ID;
    const wantProduct = env.LEMONSQUEEZY_PRODUCT_ID;
    if (wantStore && String(data.meta?.store_id ?? "") !== wantStore) return false;
    if (wantProduct && String(data.meta?.product_id ?? "") !== wantProduct) return false;

    return true;
  } catch {
    // Never let a licence-server hiccup break a paying user's rewrite.
    return false;
  }
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
      model: MODEL,
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
