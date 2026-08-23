const BASE_URL = "http://127.0.0.1:8080";

export class ApiError extends Error {
	status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const response = await fetch(`${BASE_URL}${path}`, {
		...options,
		headers: {
			"content-type": "application/json",
			...options.headers
		}
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({ error: response.statusText }));
		throw new ApiError(response.status, body.error ?? "request failed");
	}

	return response.json();
}

export type RegisterResponse = {
	username: string;
	password: string;
};

export type LoginResponse = {
	token: string;
	expires_at: string;
};

export type MeResponse = {
	username: string;
};

export function register(username: string) {
	return request<RegisterResponse>("/auth/register", {
		method: "POST",
		body: JSON.stringify({ username })
	});
}

export function login(username: string, password: string) {
	return request<LoginResponse>("/auth/login", {
		method: "POST",
		body: JSON.stringify({ username, password })
	});
}

export function me(token: string) {
	return request<MeResponse>("/auth/me", {
		headers: { authorization: `Bearer ${token}` }
	});
}
