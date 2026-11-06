import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  capcutRoots,
  readCapCutDraft,
  scanCapCutProjects,
  serveLocalMedia,
} from './server/capcut.mjs';
import {
  aiStatus,
  aiToken,
  bridgeResult,
  bridgeState,
  bridgeTake,
  startAiBridge,
} from './server/ai-bridge.mjs';

const root = fileURLToPath(new URL('.', import.meta.url));
const port = Number(process.env.PORT || 8787);

const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.webp': 'image/webp',
  '.gif': 'image/gif',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
  '.mp4': 'video/mp4',
  '.mov': 'video/quicktime',
  '.webm': 'video/webm',
  '.mp3': 'audio/mpeg',
  '.wav': 'audio/wav',
  '.m4a': 'audio/mp4',
  '.ogg': 'audio/ogg',
};

function json(res, status, body) {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(payload),
    'Cache-Control': 'no-store',
  });
  res.end(payload);
}

function readBody(req, limit = 4_000_000) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => {
      body += chunk;
      if (body.length > limit) {
        reject(new Error('Cuerpo demasiado grande'));
        req.destroy();
      }
    });
    req.on('end', () => {
      if (!body) return resolve({});
      try { resolve(JSON.parse(body)); } catch { reject(new Error('JSON no válido')); }
    });
    req.on('error', reject);
  });
}

async function handleApi(req, res, url) {
  const { pathname } = url;

  if (pathname === '/api/img' && req.method === 'GET') {
    const target = url.searchParams.get('u') || '';
    try {
      const remote = new URL(target);
      if (!/(^|\.)ibyteimg\.com$/i.test(remote.hostname)) {
        res.writeHead(403, { 'Content-Type': 'text/plain; charset=utf-8' });
        res.end('host no permitido');
        return true;
      }
      const upstream = await fetch(remote.href);
      if (!upstream.ok) {
        res.writeHead(502, { 'Content-Type': 'text/plain; charset=utf-8' });
        res.end('no se pudo descargar');
        return true;
      }
      const buffer = Buffer.from(await upstream.arrayBuffer());
      res.writeHead(200, {
        'Content-Type': upstream.headers.get('content-type') || 'image/webp',
        'Cache-Control': 'public, max-age=86400',
      });
      res.end(buffer);
    } catch {
      res.writeHead(400, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('url no válida');
    }
    return true;
  }

  if (pathname === '/api/capcut/roots' && req.method === 'GET') {
    json(res, 200, { ok: true, roots: capcutRoots });
    return true;
  }

  if (pathname === '/api/capcut/projects' && req.method === 'GET') {
    try {
      const projects = await scanCapCutProjects();
      json(res, 200, { ok: true, projects, roots: capcutRoots });
    } catch (error) {
      json(res, 500, { ok: false, error: error?.message || 'No se pudo leer CapCut' });
    }
    return true;
  }

  if (pathname === '/api/capcut/import' && req.method === 'POST') {
    try {
      const body = await readBody(req);
      const id = String(body.id || '');
      if (!id) {
        json(res, 400, { ok: false, error: 'Falta el id del proyecto de CapCut' });
        return true;
      }
      const project = await readCapCutDraft(id);
      json(res, 200, {
        ok: true,
        project,
        warnings: project.warnings || [],
        mediaCount: project.media?.length || 0,
      });
    } catch (error) {
      json(res, 400, { ok: false, error: error?.message || 'No se pudo importar el proyecto' });
    }
    return true;
  }

  if (pathname === '/api/capcut/media' && req.method === 'GET') {
    const encoded = url.searchParams.get('path') || '';
    let target = '';
    try { target = Buffer.from(encoded, 'base64url').toString('utf8'); } catch { target = ''; }
    if (!target) {
      res.writeHead(400).end('Bad request');
      return true;
    }
    await serveLocalMedia(req, res, target);
    return true;
  }

  if (pathname === '/api/capcut/cover' && req.method === 'GET') {
    const encoded = url.searchParams.get('path') || '';
    let target = '';
    try { target = Buffer.from(encoded, 'base64url').toString('utf8'); } catch { target = ''; }
    if (!target) {
      res.writeHead(400).end('Bad request');
      return true;
    }
    // Los covers detectados por el escáner se sirven como media permitida.
    const { allowMedia } = await import('./server/capcut.mjs');
    try {
      const info = await stat(target);
      if (!info.isFile()) {
        res.writeHead(404).end('Not found');
        return true;
      }
      allowMedia(target);
    } catch {
      res.writeHead(404).end('Not found');
      return true;
    }
    await serveLocalMedia(req, res, target);
    return true;
  }

  if (pathname === '/api/ai/status' && req.method === 'GET') {
    json(res, 200, {
      ...aiStatus({ includeToken: true }),
      app: `http://127.0.0.1:${port}`,
      roots: capcutRoots,
    });
    return true;
  }

  if (pathname === '/api/ai/poll' && req.method === 'POST') {
    try {
      const body = await readBody(req);
      const clientId = String(body.clientId || '').slice(0, 120);
      if (!clientId) {
        json(res, 400, { ok: false, error: 'Falta clientId' });
        return true;
      }
      if (body.state) bridgeState(clientId, body.state, Number(body.startedAt) || 0);
      const command = bridgeTake(clientId);
      json(res, 200, {
        ok: true,
        connected: true,
        command,
        serverTime: Date.now(),
      });
    } catch (error) {
      json(res, 400, { ok: false, error: error?.message || 'Solicitud no válida' });
    }
    return true;
  }

  if (pathname === '/api/ai/result' && req.method === 'POST') {
    try {
      const body = await readBody(req);
      const clientId = String(body.clientId || '').slice(0, 120);
      const id = String(body.id || '').slice(0, 120);
      if (!clientId || !id) {
        json(res, 400, { ok: false, error: 'Faltan clientId o id' });
        return true;
      }
      const ok = bridgeResult(clientId, id, {
        ok: body.ok !== false,
        result: body.result ?? null,
        error: body.error || '',
      });
      json(res, 200, { ok, id });
    } catch (error) {
      json(res, 400, { ok: false, error: error?.message || 'Solicitud no válida' });
    }
    return true;
  }

  if (pathname === '/api/ai/state' && req.method === 'POST') {
    try {
      const body = await readBody(req);
      const clientId = String(body.clientId || '').slice(0, 120);
      if (!clientId) {
        json(res, 400, { ok: false, error: 'Falta clientId' });
        return true;
      }
      bridgeState(clientId, body.state ?? null, Number(body.startedAt) || 0);
      json(res, 200, { ok: true, at: Date.now() });
    } catch (error) {
      json(res, 400, { ok: false, error: error?.message || 'Solicitud no válida' });
    }
    return true;
  }

  return false;
}

createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://${req.headers.host || '127.0.0.1'}`);
    if (url.pathname.startsWith('/api/')) {
      const handled = await handleApi(req, res, url);
      if (handled) return;
      json(res, 404, { ok: false, error: 'API no encontrada' });
      return;
    }
    let pathname = decodeURIComponent(url.pathname);
    if (pathname === '/') pathname = '/index.html';
    const filePath = normalize(join(root, pathname));
    if (!filePath.startsWith(root.endsWith(sep) ? root : root + sep)) {
      res.writeHead(403).end('Forbidden');
      return;
    }
    const info = await stat(filePath);
    if (info.isDirectory()) {
      res.writeHead(404).end('Not found');
      return;
    }
    const body = await readFile(filePath);
    res.writeHead(200, {
      'Content-Type': types[extname(filePath).toLowerCase()] || 'application/octet-stream',
      'Content-Length': body.length,
      'Cache-Control': 'no-store',
    });
    res.end(body);
  } catch (error) {
    const code = error && error.code === 'ENOENT' ? 404 : 500;
    res.writeHead(code, { 'Content-Type': 'text/plain; charset=utf-8' });
    res.end(code === 404 ? 'Not found' : 'Internal error');
  }
}).listen(port, '127.0.0.1', async () => {
  console.log(`cupcraft listo en http://127.0.0.1:${port}`);
  try {
    const bridge = await startAiBridge();
    console.log(`Puente IA en http://127.0.0.1:${bridge.port} (token: ${aiToken})`);
  } catch (error) {
    console.error(`No se pudo iniciar el puente IA: ${error?.message || error}`);
  }
});
