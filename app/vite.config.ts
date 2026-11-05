import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  server: { port: 1420, strictPort: true },
  define: { "import.meta.vitest": "undefined" },
  test: {
    includeSource: [
      "src/lib/ponte.ts",
      "src/lib/ponte.mock.ts",
      "src/lib/estado.svelte.ts",
    ],
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
  },
});
