import { base } from "$app/paths";

const cache = new Map<string, HTMLAudioElement>();

function load(name: string): HTMLAudioElement {
	let audio = cache.get(name);
	if (!audio) {
		audio = new Audio(`${base}/sounds/${name}.mp3`);
		audio.preload = "auto";
		cache.set(name, audio);
	}
	return audio;
}

// WKWebView / WebKitGTK (the Tauri webview on macOS/Linux) gate the FIRST
// programmatic play() behind a real user gesture, and a `new Audio()` created
// and play()ed later - e.g. when a call starts - often doesn't count even
// though a click happened earlier. Unlock every sound element once, inside
// the first pointer/key gesture on the document, by play()/pause()-ing it
// muted; after that the ringback's own play() is treated as a resume.
let unlocked = false;
export function installAudioUnlock() {
	if (unlocked || typeof document === "undefined") return;
	const unlock = () => {
		if (unlocked) return;
		unlocked = true;
		for (const name of ["call", "notification"]) {
			const el = load(name);
			const prevMuted = el.muted;
			el.muted = true;
			el.load();
			void el
				.play()
				.then(() => {
					el.pause();
					el.currentTime = 0;
					el.muted = prevMuted;
				})
				.catch(() => {
					el.muted = prevMuted;
				});
		}
		primeRing();
		document.removeEventListener("pointerdown", unlock, true);
		document.removeEventListener("keydown", unlock, true);
	};
	document.addEventListener("pointerdown", unlock, true);
	document.addEventListener("keydown", unlock, true);
}

function play(name: string) {
	try {
		const audio = load(name);
		audio.currentTime = 0;
		void audio.play().catch(() => {
			// Autoplay blocked (no user gesture yet this session) or the
			// element failed - not worth surfacing, it's just a sound effect.
		});
	} catch {
		// Audio unsupported in this environment - no-op.
	}
}

export function playNotificationSound() {
	play("notification");
}

export function playCallSound() {
	play("call");
}

// Looping ringback while you're alone in a call waiting for someone to join.
// One persistent element, reused across calls, so installAudioUnlock() can
// prime this exact node - a fresh `new Audio()` per call would be a
// different, still-locked element in the Tauri webview.
let ringEl: HTMLAudioElement | null = null;
let ringing = false;
let ringPlay: Promise<void> | null = null;

function getRingEl(): HTMLAudioElement {
	if (!ringEl) {
		ringEl = new Audio(`${base}/sounds/call.mp3`);
		ringEl.loop = true;
		ringEl.preload = "auto";
	}
	return ringEl;
}

// Keep the ring element in the set that the first-gesture unlock primes.
function primeRing() {
	const el = getRingEl();
	el.muted = true;
	el.loop = false;
	void el
		.play()
		.then(() => {
			el.pause();
			el.currentTime = 0;
			el.muted = false;
			el.loop = true;
		})
		.catch(() => {
			el.muted = false;
			el.loop = true;
		});
}

export function startCallRing() {
	if (ringing) return;
	ringing = true;
	try {
		const el = getRingEl();
		el.loop = true;
		el.currentTime = 0;
		ringPlay = el
			.play()
			.then(() => {
				// Got stopped between the call and play() settling.
				if (!ringing) {
					el.pause();
					el.currentTime = 0;
				}
			})
			.catch(() => {
				// Locked webview: retry once on the next user gesture.
				if (typeof document !== "undefined") {
					const retry = () => {
						document.removeEventListener("pointerdown", retry, true);
						document.removeEventListener("keydown", retry, true);
						if (ringing) el.play().catch(() => {});
					};
					document.addEventListener("pointerdown", retry, true);
					document.addEventListener("keydown", retry, true);
				}
			});
	} catch {
		ringing = false;
	}
}

export function stopCallRing() {
	if (!ringing && !ringPlay) return;
	ringing = false;
	const el = ringEl;
	if (!el) return;
	const kill = () => {
		try {
			el.pause();
			el.currentTime = 0;
		} catch {
			/* no-op */
		}
	};
	kill();
	// play() can still be resolving; pause() issued before it settles is
	// ignored on mobile, so kill again once it does.
	ringPlay?.finally(kill);
	ringPlay = null;
}
