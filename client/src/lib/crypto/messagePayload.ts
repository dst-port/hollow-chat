import type { EncryptedAttachmentMeta } from "./attachment";

const PAYLOAD_MARK = "hcMSG1:";

export type MessagePayload = {
	text: string;
	attachment?: EncryptedAttachmentMeta;
};

export function packPayload(text: string, attachment?: EncryptedAttachmentMeta): string {
	if (!attachment) return text;
	const payload: MessagePayload = { text, attachment };
	return PAYLOAD_MARK + JSON.stringify(payload);
}

export function unpackPayload(raw: string): MessagePayload {
	if (!raw.startsWith(PAYLOAD_MARK)) return { text: raw };
	try {
		return JSON.parse(raw.slice(PAYLOAD_MARK.length)) as MessagePayload;
	} catch {
		return { text: raw };
	}
}
