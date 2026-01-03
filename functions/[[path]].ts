interface Env {
  TURSO_DATABASE_HOST: string;
  TURSO_AUTH_TOKEN: string;
  PUBLIC_SITE_ORIGIN?: string;
}

export const onRequest: PagesFunction<Env> = async (context) => {
  const { request, env } = context;
  const url = new URL(request.url);

  // CORS preflight
  if (request.method === "OPTIONS") {
    return new Response(null, {
      headers: {
        "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*",
        "Access-Control-Allow-Methods": "POST, GET, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type, Authorization"
      }
    });
  }

  // Only handle /api/* routes
  if (!url.pathname.startsWith("/api")) {
    return context.next();
  }

  // Block training routes
  if (url.pathname.startsWith("/api/train")) {
    return new Response(JSON.stringify({ error: "Training is disabled in production" }), {
      status: 405,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*" }
    });
  }

  // Extract Turso path
  let tursoPath = url.pathname.replace("/api", "");
  if (tursoPath === "" || tursoPath === "/") {
    tursoPath = url.searchParams.get("path") || "/v2/pipeline";
  }

  let body: any = undefined;
  if (request.method === "POST") {
    try { body = await request.json(); } catch { body = {}; }
  }

  const tursoUrl = `https://${env.TURSO_DATABASE_HOST}${tursoPath}`;
  const tursoRes = await fetch(tursoUrl, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${env.TURSO_AUTH_TOKEN}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(body || {})
  });

  const resp = new Response(tursoRes.body, tursoRes);
  resp.headers.set("Access-Control-Allow-Origin", env.PUBLIC_SITE_ORIGIN || "*");
  return resp;
};
