# Changelog

All notable changes to HollowChat are listed here. Dates are in UTC+3.

## v0.2.10 — 2026-08-31

### Profiles
- **Animated avatars and banners.** Upload a short MP4 or WebM (GIF already
  worked) and it plays as a muted loop on your profile, in the member list,
  and next to your messages.

### Calls
- **Screen-share picker.** Before you go live you now pick the source
  (whole screen / window / tab) and see a live preview, tune resolution,
  frame rate and smoothness-vs-clarity on the running capture, and choose
  whether to include audio — then "Go Live" hands that exact stream to the
  call. The browser still asks which surface to grab; everything around it
  is ours now.
- **The shared screen is its own block** in the call, sized to the picture
  with no dead letterbox space; people are compact cards beside it, and a
  screen tile has an expand button for true fullscreen.
- **Desktop call audio fixed.** On Linux the packaged app couldn't reach
  PipeWire from its sandbox, so remote voice never played; it now routes
  through the PulseAudio-compat path. The ring/notification sounds work
  again there too.

## v0.2.9 — 2026-08-30

### Sync & notifications
- **Real-time account sync across devices.** Joining or leaving a server,
  friend requests and their replies, new DMs and group-DM changes now show up
  on every device you're signed in on immediately, no reload.
- **Live messages.** New, edited, and deleted messages arrive over the
  gateway connection the moment they happen instead of on a few-second poll.
- **Unread markers.** Servers, channels, and DMs now show an unread pill and a
  count for messages you haven't seen, kept in sync across devices, with the
  DM total mirrored in the browser tab title.
- **Push notifications (web).** Opt in from Settings → Notifications to get an
  OS notification for a DM or an @mention while HollowChat is closed. The
  notification says nothing but "You have a new message" — no sender, no
  channel, no content — so nothing leaks on a lock screen. Clicking it opens
  straight to that conversation.
- **Installable web app.** HollowChat at /app can now be installed as a
  standalone app (its own window, dock/home-screen icon) from the browser.

### Fixes
- **Screen share in a DM call now fits the call panel** instead of stretching
  off the bottom of the screen and pushing the call controls out of view. It
  scales to the available space and letterboxes any screen ratio; the shared
  audio also plays on its own track now.
- **Ringback tone now plays reliably in the desktop app.** The packaged
  webview was blocking the sound that fires when a DM call starts — it's now
  unlocked on your first click in the app, with a retry if the first attempt
  is still blocked.
- **Server icons now fill the whole circle.** The rail button kept a bit of
  default padding, so an icon sat inset with a sliver of sidebar colour
  showing around it instead of reaching the edge.

## v0.2.7 — 2026-08-30

### Calls
- **Redesigned call stage** with Discord-style participant tiles — each person
  gets a card with their avatar, name, and a green ring while they're talking.
- **"Calling…" screen for DM calls.** When you start a call with a friend, you
  now see your avatar next to theirs with rings rippling outward while it
  rings, instead of a blank audio-only view.
- **Ringback tone** now plays reliably when you start a DM call (it was being
  blocked by the browser's autoplay rules) and stops cleanly the moment the
  call ends or you hang up.
- Ringback and the 2-minute "alone in the call" auto-leave now apply **only to
  DM calls**, not to joining a server voice channel.
- **"Started a call" line in chat.** A DM call now leaves a system line in the
  conversation, and when the call ends it updates to show how long it lasted
  ("a few seconds", "5 minutes", "2 hours").
- **Screen-share quality picker.** Before you go live you can now choose
  resolution, frame rate, motion-vs-detail optimisation, and whether to share
  audio.

### Chat & profiles
- **Typing indicators** — see who's currently typing in a channel or DM.
- **Nameplate fonts.** Pick a distinctive font for your display name that
  everyone sees (HollowChatter perk).
- **Profile connections** are now shown as compact rows instead of bulky
  bordered cards.
- HollowChat now **remembers the last server/channel (and DM) you had open**
  and restores it on reload.
- The ringing animation stays visible even with "reduce motion" enabled.

### HollowChatter
- **Perks page** with a clear Free vs HollowChatter comparison table in
  Settings → Billing.
- **One-off support.** You can now chip in a one-time amount of your choosing
  toward hosting — separate from the HollowChatter subscription, nothing in
  the app is paywalled.

### Project & website
- New **"About the project"** section on hollowchat.org — it's built, run, and
  paid for by one person; bug reports and optional support both help.
- Full **Community Guidelines** page covering what's allowed in public server
  communities versus private end-to-end-encrypted chats.
- FAQ and footer now state plainly that HollowChat is an independent project,
  **not affiliated with or endorsed by Discord**.

## v0.2.6 — 2026-08-30

- Call/notification sounds ship as MP3 (OGG doesn't play in Safari or the
  in-app webview on iOS).
- Ringback loops while you're alone in a call and auto-leaves after 2 minutes.
- Fixed the ringback continuing to loop after hang-up on mobile.
- Profile colours apply as a Discord-style theme tint on your profile.
- DM profile panel and server member list now fill the full column height
  instead of collapsing.
- Server icon upload gets a pan-and-zoom crop dialog.

## v0.2.5 — 2026-08-30

- Custom dropdown menus throughout the app (replacing native `<select>`).
- Bigger avatars in profile popovers; square server icons.
- Server boosts are now stackable.
- In-app moderation report decryption using a session-only staff key.

## v0.2.4 — 2026-08-30

- **Full internationalisation** — the web client is now translated into 32
  languages.
- Group DMs: create, rename, leave, add members; end-to-end encrypted with
  Sender Keys.
- Client-side message search across your full history (the server stays
  encryption-blind — decryption and search happen on your device).
- Five built-in colour presets (Midnight Blue, Forest, Crimson, Lavender,
  Paper Light).
- Fullscreen toggle and per-participant connection-quality indicators on the
  call stage.
- Multi-file attachments via drag, paste, or picker, with an upload-progress
  banner and a custom player for voice/audio files.
- Interface font picker in Settings → Appearance.
- Optional Bunny CDN storage backend for attachments.
