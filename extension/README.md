# HollowChat Media Presence

Chromium browser extension that reports what you're watching or listening to
on YouTube, SoundCloud, Spotify (web player) and Twitch to the HollowChat
desktop app, so it can show up as Rich Presence to your friends — the same
way a game's Discord-RPC integration would.

It never talks to any HollowChat server directly. It only posts to
`http://127.0.0.1:47821/presence`, a small HTTP server the desktop app runs
locally (see `client/src-tauri/src/media_bridge.rs`). If the app isn't
running, the extension just silently does nothing.

Whether this is ever shown to friends is controlled by the existing
**Show activity status** toggle in HollowChat's Settings → Privacy & Safety
— same switch that already gates game Rich Presence.

## Install (unpacked, dev mode)

1. Open `chrome://extensions` (or `edge://extensions`, `brave://extensions`, ...)
2. Enable **Developer mode** (top right)
3. Click **Load unpacked** and select this `extension/` folder
4. Open a video on YouTube, a track on SoundCloud, etc. — HollowChat picks
   it up within a few seconds

## How detection works

Each content script first tries the page's `navigator.mediaSession`
metadata (what YouTube/SoundCloud/Spotify set for OS media key/lock-screen
integration) — that's accurate and doesn't depend on the site's DOM
structure. Twitch doesn't set it, so that one reads the stream title/channel
from the page directly, with a best-effort fallback if Twitch changes their
markup.

Presence clears automatically ~20 seconds after the tab stops sending
updates (closed, navigated away, or the browser closed).
