// Cover art comes from Steam's own public CDN (no API key needed) - the URL is
// just https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg
// for a real Steam app ID. Only games actually on Steam are listed here, so
// every cover is a real, verifiable image rather than a placeholder.
export type CatalogGame = {
	id: string;
	name: string;
	appid: number;
};

export const GAME_CATALOG: CatalogGame[] = [
	{ id: "cs2", name: "Counter-Strike 2", appid: 730 },
	{ id: "dota2", name: "Dota 2", appid: 570 },
	{ id: "pubg", name: "PUBG: BATTLEGROUNDS", appid: 578080 },
	{ id: "gta5", name: "Grand Theft Auto V", appid: 271590 },
	{ id: "rust", name: "Rust", appid: 252490 },
	{ id: "tf2", name: "Team Fortress 2", appid: 440 },
	{ id: "apex", name: "Apex Legends", appid: 1172470 },
	{ id: "terraria", name: "Terraria", appid: 105600 },
	{ id: "stardew", name: "Stardew Valley", appid: 413150 },
	{ id: "amongus", name: "Among Us", appid: 945360 },
	{ id: "fallguys", name: "Fall Guys", appid: 1097150 },
	{ id: "lethalcompany", name: "Lethal Company", appid: 1966720 },
	{ id: "palworld", name: "Palworld", appid: 1623730 },
	{ id: "goosegooseduck", name: "Goose Goose Duck", appid: 1568590 },
	{ id: "vrchat", name: "VRChat", appid: 438100 },
	{ id: "deadlock", name: "Deadlock", appid: 1422450 },
	{ id: "bg3", name: "Baldur's Gate 3", appid: 1086940 },
	{ id: "eldenring", name: "Elden Ring", appid: 1245620 },
	{ id: "cyberpunk", name: "Cyberpunk 2077", appid: 1091500 },
	{ id: "witcher3", name: "The Witcher 3", appid: 292030 },
	{ id: "portal2", name: "Portal 2", appid: 620 },
	{ id: "l4d2", name: "Left 4 Dead 2", appid: 550 },
	{ id: "rocketleague", name: "Rocket League", appid: 252950 },
	{ id: "valheim", name: "Valheim", appid: 892970 },
	{ id: "ark", name: "ARK: Survival Evolved", appid: 346110 },
	{ id: "dayz", name: "DayZ", appid: 221100 },
	{ id: "seaofthieves", name: "Sea of Thieves", appid: 1172620 },
	{ id: "nomanssky", name: "No Man's Sky", appid: 275850 },
	{ id: "hollowknight", name: "Hollow Knight", appid: 367520 },
	{ id: "hades", name: "Hades", appid: 1145360 },
	{ id: "dbd", name: "Dead by Daylight", appid: 381210 },
	{ id: "phasmophobia", name: "Phasmophobia", appid: 739630 },
	{ id: "ittakestwo", name: "It Takes Two", appid: 1426210 },
	{ id: "sonsoftheforest", name: "Sons of the Forest", appid: 1326470 },
	{ id: "warframe", name: "Warframe", appid: 230410 },
	{ id: "destiny2", name: "Destiny 2", appid: 1085660 }
];

export function coverUrl(appid: number): string {
	return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900.jpg`;
}
