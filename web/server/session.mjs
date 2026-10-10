import { randomBytes, createCipheriv, createDecipheriv } from "node:crypto";

export const COOKIE = "__Host-whakoom_session";
export const MAX_AGE = 7200;
function key(secret) {
  if (!/^[a-f0-9]{64}$/i.test(secret ?? ""))
    throw new Error("El servidor necesita configurar WHAKOOM_SESSION_KEY.");
  return Buffer.from(secret, "hex");
}
export function sealSession(data, secret, now = Date.now()) {
  const nonce = randomBytes(12);
  const cipher = createCipheriv("aes-256-gcm", key(secret), nonce);
  cipher.setAAD(Buffer.from(COOKIE));
  const encrypted = Buffer.concat([
    cipher.update(JSON.stringify({ ...data, expires: now + MAX_AGE * 1000 })),
    cipher.final(),
  ]);
  const token = Buffer.concat([nonce, cipher.getAuthTag(), encrypted]).toString(
    "base64url",
  );
  if (token.length > 3800)
    throw new Error("La sesión supera el tamaño permitido.");
  return token;
}
export function openSession(token, secret, now = Date.now()) {
  if (!token || token.length > 3800 || !/^[\w-]+$/.test(token)) return null;
  try {
    const bytes = Buffer.from(token, "base64url");
    const cipher = createDecipheriv(
      "aes-256-gcm",
      key(secret),
      bytes.subarray(0, 12),
    );
    cipher.setAAD(Buffer.from(COOKIE));
    cipher.setAuthTag(bytes.subarray(12, 28));
    const data = JSON.parse(
      Buffer.concat([
        cipher.update(bytes.subarray(28)),
        cipher.final(),
      ]).toString("utf8"),
    );
    return data.expires > now &&
      data.expires <= now + MAX_AGE * 1000 &&
      /^[A-Za-z0-9_-]{1,80}$/.test(data.user?.username ?? "") &&
      typeof data.cookie === "string" &&
      !/[\r\n]/.test(data.cookie)
      ? data
      : null;
  } catch {
    return null;
  }
}
export function sessionCookie(token = "") {
  return `${COOKIE}=${token}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=${token ? MAX_AGE : 0}`;
}
export function tokenFrom(request) {
  return (request.headers.get("cookie") ?? "")
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(`${COOKIE}=`))
    ?.slice(COOKIE.length + 1);
}
