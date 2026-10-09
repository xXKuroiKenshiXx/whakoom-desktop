export const ITERATIONS = 600000;
const encoder = new TextEncoder();
const encode = (bytes) => {
  let text = "";
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
};
const decode = (text) => {
  if (typeof text !== "string" || text.length > 90000000)
    throw new Error("Respaldo demasiado grande o inválido.");
  return Uint8Array.from(atob(text), (character) => character.charCodeAt(0));
};
export function validateEnvelope(envelope) {
  if (
    envelope?.format !== "whakoom-backup-v1" ||
    envelope.iterations !== ITERATIONS ||
    Object.keys(envelope).sort().join() !==
      "ciphertext,format,iterations,nonce,salt"
  ) {
    throw new Error("Formato de respaldo cifrado no compatible.");
  }
  const salt = decode(envelope.salt),
    nonce = decode(envelope.nonce),
    ciphertext = decode(envelope.ciphertext);
  if (salt.length !== 16 || nonce.length !== 12 || ciphertext.length < 16)
    throw new Error("Parámetros de cifrado inválidos.");
  return { salt, nonce, ciphertext };
}
async function derive(password, salt) {
  const material = await crypto.subtle.importKey(
    "raw",
    encoder.encode(password),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", hash: "SHA-256", salt, iterations: ITERATIONS },
    material,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}
export async function createKey(password) {
  if ([...password].length < 12)
    throw new Error("Usá una contraseña de al menos 12 caracteres.");
  const salt = crypto.getRandomValues(new Uint8Array(16));
  return { key: await derive(password, salt), salt };
}
export async function seal(data, session) {
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const plaintext = encoder.encode(JSON.stringify(data));
  try {
    const ciphertext = await crypto.subtle.encrypt(
      { name: "AES-GCM", iv: nonce },
      session.key,
      plaintext,
    );
    return {
      format: "whakoom-backup-v1",
      iterations: ITERATIONS,
      salt: encode(session.salt),
      nonce: encode(nonce),
      ciphertext: encode(new Uint8Array(ciphertext)),
    };
  } finally {
    plaintext.fill(0);
  }
}
export async function open(envelope, password) {
  const { salt, nonce, ciphertext } = validateEnvelope(envelope);
  const key = await derive(password, salt);
  let plaintext;
  try {
    plaintext = new Uint8Array(
      await crypto.subtle.decrypt(
        { name: "AES-GCM", iv: nonce },
        key,
        ciphertext,
      ),
    );
    return {
      data: JSON.parse(
        new TextDecoder("utf-8", { fatal: true }).decode(plaintext),
      ),
      session: { key, salt },
    };
  } catch {
    throw new Error("Contraseña incorrecta o respaldo dañado.");
  } finally {
    plaintext?.fill(0);
  }
}
