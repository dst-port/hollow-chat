(function () {
	const POLL_MS = 4000;

	function sourceForHost(hostname) {
		if (hostname.endsWith("youtube.com")) return "youtube";
		if (hostname.endsWith("soundcloud.com")) return "soundcloud";
		if (hostname === "open.spotify.com") return "spotify";
		if (hostname.endsWith("twitch.tv")) return "twitch";
		return null;
	}

	const source = sourceForHost(location.hostname);
	if (!source) return;

	function anyMediaPlaying() {
		const media = document.querySelectorAll("video, audio");
		for (const el of media) {
			if (!el.paused && !el.ended && el.currentTime > 0) return true;
		}
		return false;
	}

	function fromMediaSession() {
		const meta = navigator.mediaSession && navigator.mediaSession.metadata;
		if (!meta || !meta.title) return null;
		return {
			title: meta.title,
			subtitle: meta.artist || null,
			playing: anyMediaPlaying()
		};
	}

	function fromYouTube() {
		const bySession = fromMediaSession();
		if (bySession) return bySession;

		const title = document.title.replace(/\s*-\s*YouTube\s*$/, "").trim();
		if (!title) return null;
		const channel = document.querySelector(
			"ytd-channel-name yt-formatted-string a, #owner #channel-name a"
		);
		return {
			title,
			subtitle: channel ? channel.textContent.trim() : null,
			playing: anyMediaPlaying()
		};
	}

	function fromSoundCloud() {
		const bySession = fromMediaSession();
		if (bySession) return bySession;

		const titleEl = document.querySelector(
			".playbackSoundBadge__titleLink, .playbackSoundBadge__title"
		);
		if (!titleEl) return null;
		const authorEl = document.querySelector(".playbackSoundBadge__lightLink");
		const playing = document.querySelector(".playControls__play")
			? document.querySelector(".playControls__play").classList.contains("playing")
			: anyMediaPlaying();
		return {
			title: titleEl.textContent.trim(),
			subtitle: authorEl ? authorEl.textContent.trim() : null,
			playing
		};
	}

	function fromSpotify() {
		const bySession = fromMediaSession();
		if (bySession) return bySession;

		const titleEl = document.querySelector('[data-testid="context-item-info-title"]');
		if (!titleEl) return null;
		const subtitleEl = document.querySelector('[data-testid="context-item-info-subtitles"]');
		const playBtn = document.querySelector('[data-testid="control-button-playpause"]');
		const playing = playBtn ? playBtn.getAttribute("aria-label") === "Pause" : anyMediaPlaying();
		return {
			title: titleEl.textContent.trim(),
			subtitle: subtitleEl ? subtitleEl.textContent.trim() : null,
			playing
		};
	}

	function fromTwitch() {
		const titleEl = document.querySelector('[data-a-target="stream-title"]');
		const channelEl = document.querySelector('h1[class*="channel"], a[data-a-target="watch-live-nav"]');
		if (!titleEl && !channelEl) return null;
		const channel =
			(channelEl && channelEl.textContent.trim()) ||
			location.pathname.replace(/^\//, "").split("/")[0];
		return {
			title: channel || "a stream",
			subtitle: titleEl ? titleEl.textContent.trim() : null,
			playing: anyMediaPlaying() || document.querySelectorAll("video").length > 0
		};
	}

	const readers = {
		youtube: fromYouTube,
		soundcloud: fromSoundCloud,
		spotify: fromSpotify,
		twitch: fromTwitch
	};

	let lastSentKey = null;

	function report(state) {
		const key = JSON.stringify(state);
		if (key === lastSentKey) return;
		lastSentKey = key;
		chrome.runtime.sendMessage({ type: "hollowchat-presence", source, ...state }).catch(() => {});
	}

	function tick() {
		const reader = readers[source];
		const state = reader ? reader() : null;
		if (state && state.title) {
			report({ title: state.title, subtitle: state.subtitle, playing: !!state.playing });
		} else {
			report({ title: "", subtitle: null, playing: false });
		}
	}

	const interval = setInterval(tick, POLL_MS);
	tick();

	window.addEventListener("pagehide", () => {
		clearInterval(interval);
		chrome.runtime
			.sendMessage({ type: "hollowchat-presence", source, title: "", subtitle: null, playing: false })
			.catch(() => {});
	});
})();
