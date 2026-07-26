import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const port = Number(process.env.TAURI_BUNDLE_PORT || 3000);
const root = join(dirname(fileURLToPath(import.meta.url)), '..', 'bundle');
const types = { '.css': 'text/css', '.html': 'text/html', '.ico': 'image/x-icon', '.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' };

if (!existsSync(root)) throw new Error('POS bundle is missing. Run yarn prepare:bundle first.');

createServer((request, response) => {
  const pathname = decodeURIComponent(new URL(request.url || '/', `http://localhost:${port}`).pathname);
  const relative = pathname === '/' ? 'en.html' : pathname.replace(/^\/+/, '');
  const path = normalize(join(root, relative));
  const htmlPath = `${path}.html`;
  const file = path.startsWith(root) && existsSync(path) && statSync(path).isFile()
    ? path
    : path.startsWith(root) && existsSync(htmlPath) && statSync(htmlPath).isFile()
      ? htmlPath
      : join(root, 'en.html');
  response.writeHead(200, { 'Content-Type': types[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
  createReadStream(file).pipe(response);
}).listen(port, '127.0.0.1', () => console.log(`Serving POS bundle at http://127.0.0.1:${port}`));
