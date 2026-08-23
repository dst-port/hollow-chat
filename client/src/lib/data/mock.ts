export type ChannelType = "text" | "voice";

export type Channel = {
	id: string;
	name: string;
	type: ChannelType;
};

export type ServerEntry = {
	id: string;
	name: string;
	initials: string;
	channels: Channel[];
};

export type Message = {
	id: string;
	author: string;
	color: string;
	content: string;
	time: string;
};

export const servers: ServerEntry[] = [
	{
		id: "void-raiders",
		name: "Void Raiders",
		initials: "VR",
		channels: [
			{ id: "general", name: "general", type: "text" },
			{ id: "clips", name: "clips", type: "text" },
			{ id: "strategy", name: "strategy", type: "text" },
			{ id: "lounge", name: "Lounge", type: "voice" },
			{ id: "raid-voice", name: "Raid Voice", type: "voice" }
		]
	},
	{
		id: "night-owls",
		name: "Night Owls",
		initials: "NO",
		channels: [
			{ id: "general", name: "general", type: "text" },
			{ id: "memes", name: "memes", type: "text" },
			{ id: "hangout", name: "Hangout", type: "voice" }
		]
	},
	{
		id: "speedrun-hub",
		name: "Speedrun Hub",
		initials: "SH",
		channels: [
			{ id: "general", name: "general", type: "text" },
			{ id: "pbs", name: "personal-bests", type: "text" },
			{ id: "practice", name: "Practice", type: "voice" }
		]
	}
];

export const messages: Message[] = [
	{
		id: "1",
		author: "ghostpixel",
		color: "#8b5cf6",
		content: "anyone up for a raid tonight?",
		time: "20:14"
	},
	{
		id: "2",
		author: "nullbyte",
		color: "#22c55e",
		content: "yeah, give me 10 min to finish loading in",
		time: "20:15"
	},
	{
		id: "3",
		author: "ghostpixel",
		color: "#8b5cf6",
		content: "bet. voice channel is open",
		time: "20:15"
	},
	{
		id: "4",
		author: "vex",
		color: "#f97316",
		content: "count me in too",
		time: "20:17"
	}
];

export type Member = {
	id: string;
	name: string;
	color: string;
	status: "online" | "idle" | "offline";
};

export const members: Member[] = [
	{ id: "1", name: "ghostpixel", color: "#8b5cf6", status: "online" },
	{ id: "2", name: "nullbyte", color: "#22c55e", status: "online" },
	{ id: "3", name: "vex", color: "#f97316", status: "idle" },
	{ id: "4", name: "shade", color: "#64748b", status: "offline" }
];
