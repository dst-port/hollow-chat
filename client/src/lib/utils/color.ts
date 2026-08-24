const PALETTE = [
	"#9c93c2",
	"#6fb98f",
	"#e2793f",
	"#5c9ec9",
	"#c9789e",
	"#8fae5c",
	"#c9a227",
	"#5c5566"
];

export function colorForName(name: string): string {
	let hash = 0;
	for (let i = 0; i < name.length; i++) {
		hash = (hash << 5) - hash + name.charCodeAt(i);
		hash |= 0;
	}
	return PALETTE[Math.abs(hash) % PALETTE.length];
}
