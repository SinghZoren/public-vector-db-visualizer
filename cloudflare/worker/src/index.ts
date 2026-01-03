export default {
  async fetch(request: Request, env: any): Promise<Response> {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        headers: {
          "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*",
          "Access-Control-Allow-Methods": "POST, GET, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Authorization"
        }
      });
    }

    const url = new URL(request.url);
    if (url.pathname.startsWith("/train")) {
      return new Response(JSON.stringify({ error: "Training is disabled in production" }), {
        status: 405,
        headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": env.PUBLIC_SITE_ORIGIN || "*" }
      });
    }

    // Forward to Turso by default, but allow other paths if needed
    let tursoPath = url.pathname;
    if (tursoPath === "/" || tursoPath === "/api" || tursoPath === "/api/") {
      tursoPath = url.searchParams.get("path") || "/v2/pipeline";
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
  }
} satisfies ExportedHandler;
