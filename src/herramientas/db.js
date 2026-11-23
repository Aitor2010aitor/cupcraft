const DB_NAME = 'openCUP-db';
const DB_VERSION = 1;
let dbPromise = null;

function openDb() {
  if (dbPromise) return dbPromise;
  dbPromise = new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains('projects')) {
        db.createObjectStore('projects', { keyPath: 'id' });
      }
      if (!db.objectStoreNames.contains('media')) {
        db.createObjectStore('media', { keyPath: 'id' });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  return dbPromise;
}

function run(storeName, mode, action) {
  return openDb().then((db) => new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, mode);
    const store = tx.objectStore(storeName);
    let result;
    try {
      result = action(store);
    } catch (error) {
      reject(error);
      return;
    }
    tx.oncomplete = () => resolve(result);
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error || new Error('Transacción cancelada'));
  }));
}

function requestResult(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export const db = {
  async listProjects() {
    const dbx = await openDb();
    return new Promise((resolve, reject) => {
      const tx = dbx.transaction('projects', 'readonly');
      const request = tx.objectStore('projects').getAll();
      request.onsuccess = () => resolve(
        request.result.sort((a, b) => (b.updatedAt || 0) - (a.updatedAt || 0)),
      );
      request.onerror = () => reject(request.error);
    });
  },

  getProject(id) {
    return openDb().then((dbx) => requestResult(dbx.transaction('projects', 'readonly').objectStore('projects').get(id)));
  },

  saveProject(project) {
    return run('projects', 'readwrite', (store) => store.put(project));
  },

  deleteProject(id) {
    return run('projects', 'readwrite', (store) => store.delete(id));
  },

  saveMedia(record) {
    return run('media', 'readwrite', (store) => store.put(record));
  },

  getMedia(id) {
    return openDb().then((dbx) => requestResult(dbx.transaction('media', 'readonly').objectStore('media').get(id)));
  },

  deleteMedia(id) {
    return run('media', 'readwrite', (store) => store.delete(id));
  },
};
