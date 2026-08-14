import { defineConfig } from 'vite';
import fs from 'fs';

export default defineConfig({
  root: '.',
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:7890',
        changeOrigin: true,
        ws: true
      }
    }
  },
  plugins: [
    {
      name: 'termcmd-token-provider',
      configureServer(server) {
        server.middlewares.use('/__token', (_req, res) => {
          try {
            const token = fs.readFileSync('/tmp/termcmd_token', 'utf-8').trim();
            res.setHeader('Content-Type', 'application/json');
            res.end(JSON.stringify({ token }));
          } catch {
            res.statusCode = 500;
            res.end(JSON.stringify({ error: 'Token not found' }));
          }
        });
      }
    }
  ],
  build: {
    outDir: 'dist',
    target: 'esnext',
    emptyOutDir: true
  }
});
