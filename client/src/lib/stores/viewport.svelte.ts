const MOBILE_QUERY = "(max-width: 860px)";

class ViewportStore {
	isMobile = $state(false);
	private initialized = false;

	init() {
		if (this.initialized || typeof window === "undefined") return;
		this.initialized = true;
		const mql = window.matchMedia(MOBILE_QUERY);
		this.isMobile = mql.matches;
		mql.addEventListener("change", (e) => {
			this.isMobile = e.matches;
		});
	}
}

export const viewport = new ViewportStore();
