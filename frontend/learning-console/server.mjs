import http from "node:http";
import { createReadStream, promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = Number(process.env.OSEDUC_FRONTEND_PORT || 4173);
const host = process.env.OSEDUC_FRONTEND_HOST || "127.0.0.1";
const apiBaseUrl = process.env.OSEDUC_API_BASE_URL || "http://127.0.0.1:3000";

const mimeTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"],
]);

const server = http.createServer(async (request, response) => {
  try {
    if (!request.url) {
      response.writeHead(400);
      response.end("Bad request");
      return;
    }

    const url = new URL(request.url, `http://${request.headers.host || "localhost"}`);
    if (url.pathname.startsWith("/api/")) {
      await proxyApiRequest(request, response, url);
      return;
    }

    await serveStatic(response, url.pathname);
  } catch (error) {
    console.error(error);
    response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
    response.end("Internal server error");
  }
});

async function proxyApiRequest(request, response, url) {
  const targetUrl = new URL(url.pathname.replace(/^\/api/, "") + url.search, apiBaseUrl);
  const headers = { ...request.headers };
  delete headers.host;

  const body = request.method === "GET" || request.method === "HEAD" ? undefined : request;
  let upstream;
  try {
    upstream = await fetch(targetUrl, {
      method: request.method,
      headers,
      body,
      duplex: body ? "half" : undefined,
    });
  } catch (error) {
    console.error("Backend proxy request failed", error);
    response.writeHead(502, { "content-type": "application/json; charset=utf-8" });
    response.end(
      JSON.stringify({
        error: "backend_unreachable",
        message: `Backend API is not reachable at ${apiBaseUrl}`,
      }),
    );
    return;
  }

  response.writeHead(upstream.status, {
    "content-type": upstream.headers.get("content-type") || "application/octet-stream",
  });
  if (upstream.body) {
    for await (const chunk of upstream.body) {
      response.write(chunk);
    }
  }
  response.end();
}

async function serveStatic(response, pathname) {
  const safePath = path
    .normalize(decodeURIComponent(pathname))
    .replace(/^(\.\.[/\\])+/, "");
  const requestedPath = safePath === "/" ? "/index.html" : safePath;
  const filePath = path.join(__dirname, requestedPath);
  const relative = path.relative(__dirname, filePath);

  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    response.writeHead(403);
    response.end("Forbidden");
    return;
  }

  try {
    const stat = await fs.stat(filePath);
    if (!stat.isFile()) {
      throw new Error("not a file");
    }
    response.writeHead(200, {
      "content-type": mimeTypes.get(path.extname(filePath)) || "application/octet-stream",
      "cache-control": "no-store",
    });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
  }
}

server.listen(port, host, () => {
  console.log(`OSeduc learning console: http://${host}:${port}`);
  console.log(`Proxying /api to ${apiBaseUrl}`);
});
