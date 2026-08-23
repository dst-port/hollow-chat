export type Toast = {
	id: number;
	message: string;
};

let nextId = 1;

class ToastStore {
	items = $state<Toast[]>([]);

	push(message: string, duration = 2400) {
		const id = nextId++;
		this.items.push({ id, message });
		setTimeout(() => this.dismiss(id), duration);
	}

	dismiss(id: number) {
		this.items = this.items.filter((t) => t.id !== id);
	}
}

export const toast = new ToastStore();
