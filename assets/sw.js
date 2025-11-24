var cacheName = "nessie-pwa-" + (typeof __BUILD_ID__ !== 'undefined' ? __BUILD_ID__ : 'development');
var filesToCache = ["./", "./index.html", "./nessie.js", "./nessie_bg.wasm"];

self.addEventListener("install", function (e) {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    }),
  );
});

self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys().then(cacheNames => {
      return Promise.all(
        cacheNames.filter(name => name.startsWith('nessie-pwa-') && name !== cacheName)
                  .map(name => caches.delete(name))
      );
    })
  );
});

self.addEventListener("fetch", function (e) {
  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    }),
  );
});
