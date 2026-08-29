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
