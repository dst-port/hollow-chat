// @vitest-environment jsdom
// Pins live in localStorage, so these need a browser-ish environment.
import { describe, it, expect, beforeEach } from "vitest";
import { generateX25519KeyPair } from "./primitives";
import { toBase64 } from "./encoding";
import {
	IdentityChangedError,
	clearPeerIdentity,
	loadPeerIdentity,
	pinOrVerifyPeerIdentity,
	renameAllPeerIdentities,
	safetyNumber
} from "./peer-identity";

const alice = toBase64(generateX25519KeyPair().publicKey);
const bob = toBase64(generateX25519KeyPair().publicKey);
const mallory = toBase64(generateX25519KeyPair().publicKey);

describe("safetyNumber", () => {
	it("reads the same on both sides", () => {
		// The whole point: two people compare it out loud. If it depended on
		// who was asking, they'd never match and the check would be useless.
		expect(safetyNumber(alice, bob)).toBe(safetyNumber(bob, alice));
	});

	it("differs when either key differs", () => {
		expect(safetyNumber(alice, bob)).not.toBe(safetyNumber(alice, mallory));
	});

	it("is 12 groups of 5 digits", () => {
		const value = safetyNumber(alice, bob);
		const groups = value.split(" ");
		expect(groups).toHaveLength(12);
		for (const group of groups) expect(group).toMatch(/^\d{5}$/);
	});
});

describe("pinning", () => {
	beforeEach(() => localStorage.clear());

	it("pins on first contact and accepts the same key again", () => {
		pinOrVerifyPeerIdentity("me", "bob", bob);
		expect(loadPeerIdentity("me", "bob")).toBe(bob);
		expect(() => pinOrVerifyPeerIdentity("me", "bob", bob)).not.toThrow();
	});

	it("refuses a different key once pinned", () => {
		pinOrVerifyPeerIdentity("me", "bob", bob);
		expect(() => pinOrVerifyPeerIdentity("me", "bob", mallory)).toThrow(IdentityChangedError);
	});

	it("pins each peer separately", () => {
		pinOrVerifyPeerIdentity("me", "bob", bob);
		expect(() => pinOrVerifyPeerIdentity("me", "carol", mallory)).not.toThrow();
	});

	it("re-pins only after the pin is deliberately cleared", () => {
		pinOrVerifyPeerIdentity("me", "bob", bob);
		clearPeerIdentity("me", "bob");
		expect(() => pinOrVerifyPeerIdentity("me", "bob", mallory)).not.toThrow();
		expect(loadPeerIdentity("me", "bob")).toBe(mallory);
	});

	it("follows my own username through a rename", () => {
		// Otherwise every contact would look like an impostor after a rename.
		pinOrVerifyPeerIdentity("me", "bob", bob);
		renameAllPeerIdentities("me", "me2");
		expect(loadPeerIdentity("me2", "bob")).toBe(bob);
		expect(loadPeerIdentity("me", "bob")).toBeNull();
	});
});
