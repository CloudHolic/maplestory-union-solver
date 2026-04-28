import { defineConfig } from 'vite';
import { fileURLToPath } from "url";
import react from '@vitejs/plugin-react'
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [
    react(),
    tailwindcss()
  ],
  resolve: {
    alias: {
      '@solver/wasm': fileURLToPath(new URL('./wasm-pkg', import.meta.url)),
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  worker: {
    format: 'es'
  },
  server: {
    port: 5173,
    strictPort: false
  }
})
