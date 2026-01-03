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

  // Block training routes
  if (url.pathname.startsWith("/api/train")) {
    return new Response(JSON.stringify({ error: "Training is disabled in production" }), {
      status: 405,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*" }
    });
  }

  // Extract Turso path
  // If calling /api, we default to the pipeline
  let tursoPath = url.pathname.replace("/api", "");
  if (tursoPath === "" || tursoPath === "/") {
    tursoPath = url.searchParams.get("path") || "/v2/pipeline";
  }

  let body: any = undefined;
  if (request.method === "POST") {
    try {
      body = await request.json();
    } catch {
      body = {};
    }
  }

  // Construct the Turso URL
  // Note: env.TURSO_DATABASE_HOST should be something like 'your-db.turso.io'
  const tursoUrl = `https://${env.TURSO_DATABASE_HOST}${tursoPath}`;

  const tursoRes = await fetch(tursoUrl, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${env.TURSO_AUTH_TOKEN}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(body || {})
  });

  // Create response with original body and status
  const resp = new Response(tursoRes.body, {
    status: tursoRes.status,
    statusText: tursoRes.statusText,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*"
    }
  });

  return resp;
};
