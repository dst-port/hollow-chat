import { base } from "$app/paths";

const cache = new Map<string, HTMLAudioElement>();

function load(name: string): HTMLAudioElement {
	let audio = cache.get(name);
	if (!audio) {
		audio = new Audio(`${base}/sounds/${name}.ogg`);
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

export function startCallRing() {
	if (ring) return;
	try {
		ring = new Audio(`${base}/sounds/call.ogg`);
		ring.loop = true;
		void ring.play().catch(() => {});
	} catch {
		ring = null;
	}
}

export function stopCallRing() {
	if (!ring) return;
	try {
		ring.pause();
		ring.loop = false;
		ring.currentTime = 0;
	} catch {
		/* no-op */
	}
	ring = null;
}
