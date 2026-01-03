interface Env {
  TURSO_DATABASE_HOST: string;
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

    // Safety check: Are environment variables set?
    if (!env.TURSO_DATABASE_HOST || !env.TURSO_AUTH_TOKEN) {
      return new Response(JSON.stringify({ 
        error: "Environment Variables Missing",
        details: {
          has_host: !!env.TURSO_DATABASE_HOST,
          has_token: !!env.TURSO_AUTH_TOKEN
        },
        hint: "Go to Pages Dashboard -> Settings -> Functions -> Environment variables and add them there."
      }), {
        status: 500,
        headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
      });
    }

    // Clean the host
    const host = env.TURSO_DATABASE_HOST.replace("https://", "").replace("libsql://", "").split("/")[0];
    
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

    // Read the text body from Turso to avoid streaming issues
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
