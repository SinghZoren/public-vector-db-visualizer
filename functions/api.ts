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

  // Safety check: Are environment variables set?
  if (!env.TURSO_DATABASE_HOST || !env.TURSO_AUTH_TOKEN) {
    return new Response(JSON.stringify({ 
      error: "Cloudflare Environment Variables are missing.",
      hint: "Add TURSO_DATABASE_HOST and TURSO_AUTH_TOKEN in your Pages Dashboard -> Settings -> Functions."
    }), {
      status: 500,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }

  // Block training routes
  if (url.pathname.startsWith("/api/train")) {
    return new Response(JSON.stringify({ error: "Training is disabled in production" }), {
      status: 405,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }

  // Clean the host (remove https:// or libsql:// if the user included it by mistake)
  let host = env.TURSO_DATABASE_HOST.replace("https://", "").replace("libsql://", "").split("/")[0];
  
  // Extract Turso path
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

  const tursoUrl = `https://${host}${tursoPath}`;

  try {
    const tursoRes = await fetch(tursoUrl, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${env.TURSO_AUTH_TOKEN}`,
        "Content-Type": "application/json",
        "User-Agent": "Cloudflare-Pages-Proxy/1.0"
      },
      body: JSON.stringify(body || {})
    });

    const resp = new Response(tursoRes.body, {
      status: tursoRes.status,
      statusText: tursoRes.statusText,
      headers: {
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*"
      }
    });

    return resp;
  } catch (e: any) {
    return new Response(JSON.stringify({ error: "Proxy Fetch Failed", message: e.message }), {
      status: 502,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }
};
