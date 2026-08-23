export type ChannelType = "text" | "voice";

export type Channel = {
	id: string;
	name: string;
	type: ChannelType;
	unread?: boolean;
};

export type ServerEntry = {
	id: string;
	name: string;
	initials: string;
	channels: Channel[];
	unread?: number;
};

export type Reaction = {
	emoji: string;
	count: number;
	reacted: boolean;
};

export type Message = {
	id: string;
	author: string;
	color: string;
	content: string;
	time: string;
	reactions?: Reaction[];
	pinned?: boolean;
};

export function createServers(): ServerEntry[] {
	return [
		{
			id: "void-raiders",
			name: "Void Raiders",
			initials: "VR",
			channels: [
				{ id: "general", name: "general", type: "text" },
				{ id: "clips", name: "clips", type: "text", unread: true },
				{ id: "strategy", name: "strategy", type: "text" },
				{ id: "lounge", name: "Lounge", type: "voice" },
				{ id: "raid-voice", name: "Raid Voice", type: "voice" }
			]
		},
		{
			id: "night-owls",
			name: "Night Owls",
			initials: "NO",
			unread: 3,
			channels: [
				{ id: "general", name: "general", type: "text", unread: true },
				{ id: "memes", name: "memes", type: "text", unread: true },
				{ id: "hangout", name: "Hangout", type: "voice" }
			]
		},
		{
			id: "speedrun-hub",
			name: "Speedrun Hub",
			initials: "SH",
			unread: 12,
			channels: [
				{ id: "general", name: "general", type: "text", unread: true },
				{ id: "pbs", name: "personal-bests", type: "text" },
				{ id: "practice", name: "Practice", type: "voice" }
			]
		}
	];
}

export function createMessages(): Message[] {
	return [
		{
			id: "1",
			author: "ghostpixel",
			color: "#9c93c2",
			content: "anyone up for a raid tonight?",
			time: "20:14",
			reactions: [{ emoji: "🔥", count: 2, reacted: false }]
		},
		{
			id: "2",
			author: "nullbyte",
			color: "#6fb98f",
			content: "yeah, give me 10 min to finish loading in",
			time: "20:15"
		},
		{
			id: "3",
			author: "ghostpixel",
			color: "#9c93c2",
			content: "bet. voice channel is open",
			time: "20:15"
		},
		{
			id: "4",
			author: "vex",
			color: "#e2793f",
			content: "count me in too",
			time: "20:17"
		}
	];
}

export type Member = {
	id: string;
	name: string;
	color: string;
	status: "online" | "idle" | "offline";
};

export const members: Member[] = [
	{ id: "1", name: "ghostpixel", color: "#9c93c2", status: "online" },
	{ id: "2", name: "nullbyte", color: "#6fb98f", status: "online" },
	{ id: "3", name: "vex", color: "#e2793f", status: "idle" },
	{ id: "4", name: "shade", color: "#5c5566", status: "offline" }
];

export const EMOJI_PALETTE = ["👍", "❤️", "😂", "🔥", "🎉", "👀"];
