export async function onRequest(context) {
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
  const rawHost = env.TURSO_DATABASE_HOST || env.TURSO_DATABASE_URL;
  if (!rawHost || !env.TURSO_AUTH_TOKEN) {
    return new Response(JSON.stringify({ 
      error: "Environment Variables Missing",
      hint: "Make sure you have TURSO_DATABASE_URL and TURSO_AUTH_TOKEN set in Cloudflare Settings."
    }), {
      status: 500,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }

  // Block training routes
  if (url.pathname.includes("/train")) {
    return new Response(JSON.stringify({ error: "Training is disabled in production" }), {
      status: 405,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }

  // Clean the host
  const host = rawHost.replace("https://", "").replace("libsql://", "").split("/")[0].split("?")[0];
  
  // Extract Turso path
  let tursoPath = url.pathname.replace("/api", "");
  if (tursoPath === "" || tursoPath === "/") {
    tursoPath = url.searchParams.get("path") || "/v2/pipeline";
  }

  let body = undefined;
  if (request.method === "POST") {
    try {
      body = await request.text(); // Read as text to pass directly
    } catch {
      body = "";
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
      body: body
    });

    const resText = await tursoRes.text();

    return new Response(resText, {
      status: tursoRes.status,
      headers: {
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*"
      }
    });

  } catch (e) {
    return new Response(JSON.stringify({ 
      error: "Proxy Crash", 
      message: e.message
    }), {
      status: 500,
      headers: { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" }
    });
  }
}
