import { base } from "$app/paths";

const cache = new Map<string, HTMLAudioElement>();

function load(name: string): HTMLAudioElement {
	let audio = cache.get(name);
	if (!audio) {
		audio = new Audio(`${base}/sounds/${name}.mp3`);
		cache.set(name, audio);
	}
	return audio;
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
// Uses its own element (not the shared cache) so the one-shot join blip
// and the loop don't fight over one <audio>.
let ring: HTMLAudioElement | null = null;
let ringPlay: Promise<void> | null = null;

function killRing(el: HTMLAudioElement) {
	try {
		el.loop = false;
		el.pause();
		el.currentTime = 0;
		// Detach the source so nothing can resume it, then reset the element.
		el.removeAttribute("src");
		el.load();
	} catch {
		/* no-op */
	}
}

export function startCallRing() {
	if (ring) return;
	try {
		const el = new Audio(`${base}/sounds/call.mp3`);
		el.loop = true;
		ring = el;
		// play() resolves late on mobile; a pause() issued before it settles
		// is silently ignored, so always re-kill once it settles.
		ringPlay = el
			.play()
			.then(() => {
				if (ring !== el) killRing(el);
			})
			.catch(() => {});
	} catch {
		ring = null;
	}
}

export function stopCallRing() {
	const el = ring;
	ring = null;
	if (!el) return;
	killRing(el);
	ringPlay?.finally(() => killRing(el));
	ringPlay = null;
}
