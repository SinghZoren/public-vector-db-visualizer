interface Env {
  TURSO_DATABASE_HOST?: string;
  TURSO_DATABASE_URL?: string;
  TURSO_AUTH_TOKEN: string;
  PUBLIC_SITE_ORIGIN?: string;
}

export const onRequest: PagesFunction<Env> = async (context) => {
  try {
    const { request, env } = context;
    const url = new URL(request.url);

    // CORS preflight
    if (request.method === "OPTIONS") {
      return new Response(null, {
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "POST, GET, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Authorization"
        }
      });
    }

    // Try to find the host in either variable
    const rawHost = env.TURSO_DATABASE_HOST || env.TURSO_DATABASE_URL;

    // Safety check: Are environment variables set?
    if (!rawHost || !env.TURSO_AUTH_TOKEN) {
      return new Response(JSON.stringify({ 
        error: "Environment Variables Missing",
        details: {
          has_host: !!rawHost,
          has_token: !!env.TURSO_AUTH_TOKEN
        },
        hint: "Make sure you have TURSO_DATABASE_URL and TURSO_AUTH_TOKEN set in Cloudflare Settings."
      }), {
        status: 500,
        headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
      });
    }

    // Clean the host (remove protocol and everything after the first slash)
    const host = rawHost.replace("https://", "").replace("libsql://", "").split("/")[0].split("?")[0];
    
    // Extract path
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

    const tursoRes = await fetch(tursoUrl, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${env.TURSO_AUTH_TOKEN}`,
        "Content-Type": "application/json",
        "User-Agent": "Cloudflare-Pages-Proxy/1.0"
      },
      body: JSON.stringify(body || {})
    });

    const resText = await tursoRes.text();

    return new Response(resText, {
      status: tursoRes.status,
      headers: {
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*"
      }
    });

  } catch (e: any) {
    return new Response(JSON.stringify({ 
      error: "Proxy Crash", 
      message: e.message,
      stack: e.stack
    }), {
      status: 500,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }
};
