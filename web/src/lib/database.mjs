const openDatabase = () =>
  new Promise((resolve, reject) => {
    const request = indexedDB.open("whakoom-library", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("vault");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () =>
      reject(new Error("No se puede acceder al almacenamiento del navegador."));
  });
async function transaction(mode, action) {
  const db = await openDatabase();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction("vault", mode);
      const request = action(tx.objectStore("vault"));
      tx.oncomplete = () => resolve(request.result);
      tx.onerror = () =>
        reject(
          new Error(
            "No se pudieron guardar los datos. Exportá un respaldo antes de cerrar.",
          ),
        );
      tx.onabort = tx.onerror;
    });
  } finally {
    db.close();
  }
}
export const readVault = () =>
  transaction("readonly", (store) => store.get("library"));
export const saveVault = (envelope) =>
  transaction("readwrite", (store) => store.put(envelope, "library"));
export const deleteVault = () =>
  transaction("readwrite", (store) => store.delete("library"));
