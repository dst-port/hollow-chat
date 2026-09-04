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

// Only the web build gets a CSP. The desktop app loads the same HTML but talks
// to Tauri over its own IPC protocol, and a policy written for https:// origins
// blocks that - so it stays on Tauri's own (tauri.conf.json) handling.
const isWebBuild = !!process.env.HOLLOWCHAT_WEB_BASE;

// "hash" lets SvelteKit whitelist its own inline bootstrap script by digest,
// so we never need 'unsafe-inline' for scripts - which is the whole point.
// 'unsafe-inline' does stay on style-src: Svelte sets inline style attributes
// all over the app, and style injection isn't the risk script injection is.
const csp = {
  mode: "hash",
  directives: {
    "default-src": ["self"],
    "script-src": ["self"],
    "style-src": ["self", "unsafe-inline", "https://fonts.googleapis.com"],
    "font-src": ["self", "data:", "https://fonts.gstatic.com"],
    // Link previews and game covers legitimately point at arbitrary sites, and
    // an image source is not an execution sink.
    "img-src": ["self", "data:", "blob:", "https:"],
    "media-src": ["self", "blob:", "https:"],
    "connect-src": ["self", "https://hollowchat.org", "https://cdn.hollowchat.org", "wss://hollowchat.org"],
    "worker-src": ["self", "blob:"],
    "object-src": ["none"],
    "base-uri": ["self"],
    "form-action": ["self"],
  },
};

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
    ...(isWebBuild ? { csp } : {}),
    // Registered manually (only on web, only when the user opts into push) so
    // the Tauri build - where service workers aren't available - never tries.
    serviceWorker: {
      register: false,
    },
  },
};

export default config;
