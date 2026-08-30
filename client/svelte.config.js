// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// The desktop build (Tauri, served from a custom protocol root) needs an
// empty base path. The self-hosted web build sits at hollowchat.org/app
// alongside the landing page at /, so it needs its asset paths prefixed -
// set only via this env var, which only the web-deploy build script sets.
const base = process.env.HOLLOWCHAT_WEB_BASE ?? "";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    paths: {
      base,
    },
    // Registered manually (only on web, only when the user opts into push) so
    // the Tauri build - where service workers aren't available - never tries.
    serviceWorker: {
      register: false,
    },
  },
};

export default config;
