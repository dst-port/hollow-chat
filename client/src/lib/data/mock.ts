export type ChannelType = "text" | "voice";

export type Channel = {
	id: string;
	name: string;
	type: ChannelType;
	unread?: boolean;
	category?: string;
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
				{ id: "rules", name: "rules", type: "text", category: "📌 Important" },
				{ id: "announcements", name: "announcements", type: "text", category: "📌 Important" },
				{ id: "general", name: "general", type: "text", category: "💬 Chats" },
				{ id: "clips", name: "clips", type: "text", unread: true, category: "💬 Chats" },
				{ id: "strategy", name: "strategy", type: "text", category: "💬 Chats" },
				{ id: "lounge", name: "Lounge", type: "voice", category: "🔊 Voice" },
				{ id: "raid-voice", name: "Raid Voice", type: "voice", category: "🔊 Voice" }
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

export type Role = {
	label: string;
	color: string;
};

export type BadgeId = "supporter" | "dev-contributor" | "developer" | "owner" | "staff";

export const BADGE_META: Record<BadgeId, { label: string; description: string }> = {
	supporter: { label: "Supporter", description: "Subscribed to Hollow Chatter" },
	"dev-contributor": { label: "Hollow Chat Development", description: "Contributed to building HollowChat" },
	developer: { label: "Developer", description: "Verified app or bot developer" },
	owner: { label: "Owner", description: "Owns HollowChat" },
	staff: { label: "Hollow Staff", description: "Works at HollowChat" }
};

export type Member = {
	id: string;
	name: string;
	color: string;
	status?: "online" | "idle" | "offline";
	roles?: Role[];
	badges?: BadgeId[];
	activity?: string;
	bio?: string;
	memberSince?: string;
};

export const EMOJI_PALETTE = ["👍", "❤️", "😂", "🔥", "🎉", "👀"];
