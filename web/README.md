# Whakoom Web

La versión web de Whakoom Desktop permite comenzar con una cuenta de Whakoom, sin importar un archivo. Incluye biblioteca y deseados con carga por páginas, búsqueda y exploración del catálogo, fichas y opiniones, listas, perfiles, seguidos y seguidores. Las 21 insignias locales tienen progreso y efectos; también hay estadísticas, notas, fechas e importes por moneda, temas claro y oscuro y tutorial inicial.

## Conexión y datos

El login utiliza el formulario y la sesión de Whakoom mediante una función de servidor. No hay una API pública oficial ni OAuth: si Whakoom cambia su web, restringe la cuenta o exige una verificación, el conector puede dejar de funcionar. La aplicación informa el error y no evita esas verificaciones. Las operaciones de colección, deseados, lectura, valoraciones y opiniones sólo se muestran confirmadas después de comprobar la respuesta del servicio.

Las credenciales se transmiten al servidor de esta web por HTTPS exclusivamente para iniciar sesión. No se guardan ni se registran. La sesión queda cifrada con AES-256-GCM en una cookie HttpOnly, Secure y SameSite=Strict de dos horas. Las funciones sólo consultan el dominio permitido, verifican el origen de las escrituras, limitan el tamaño de las solicitudes y respetan las pausas HTTP 429. El login tiene un límite independiente de cinco intentos por minuto e IP.

Después del login podés elegir un respaldo cifrado en IndexedDB con una contraseña independiente, que nunca sale del navegador, o continuar sólo en memoria. Los archivos `.whakoom` son compatibles con Desktop y utilizan AES-256-GCM y PBKDF2-SHA256 con 600000 iteraciones. Sólo se pueden importar respaldos de la cuenta conectada. Las notas, fechas, importes, estado Leyendo, reacciones a comentarios, personas favoritas e insignias son datos locales: exportá un respaldo para moverlos a otro dispositivo.

Las portadas usan la caché HTTP del navegador. Las respuestas de cuenta y sesión no se guardan en el service worker ni en cachés compartidas. Las consultas se reutilizan dos minutos en memoria, sin almacenar la sesión en JavaScript. La web no sustituye todas las funciones de Desktop o de Whakoom Pro: todavía no edita configuración de perfil, crea productos, gestiona suscripciones ni publica nuevas listas. No hay ranking global de insignias.

## Desarrollo

Requiere Node.js 24.

```sh
npm ci
npm test
npm run build
npm run dev
```

`astro dev` sirve la interfaz. Para probar también las funciones, usá `netlify dev` desde esta carpeta y configurá las variables de servidor. El navegador no puede iniciar sesión contra la vista estática aislada.

## Despliegue

La carpeta `web` contiene su propio `netlify.toml`, frontend y funciones: no depende del proyecto Rust. Conectá el repositorio a Netlify con directorio base `web`, comando `npm run build` y publicación `dist`. También podés desplegar esta carpeta con la CLI. Subir sólo `dist` por arrastrar y soltar publica archivos estáticos y **no** instala las funciones de login.

En las variables de entorno del sitio, disponibles para Functions, definí:

- `WHAKOOM_SESSION_KEY`: una clave nueva de 32 bytes expresada como 64 caracteres hexadecimales. Podés generarla con `node -e "console.log(require('node:crypto').randomBytes(32).toString('hex'))"`. Guardala sólo en la configuración privada del servidor. Rotarla cierra todas las sesiones.
- `WHAKOOM_APP_ORIGIN`: opcional, la URL pública exacta sin barra final, para restringir el origen de las escrituras. Si se omite, se utiliza el origen de la solicitud de la función.

Usá HTTPS y una cuenta de hosting confiable: su administrador tiene acceso al servidor que procesa el login. No configures las claves con prefijos `PUBLIC_` ni las subas al repositorio. El archivo `.env.example` sólo contiene nombres vacíos. La configuración añade CSP, bloqueo de iframes, protección frente a detección de tipos y ausencia de acceso cruzado por CORS.

## Validación

`npm test` comprueba cifrado y compatibilidad con Desktop, cookies, caducidad, CSRF, destinos permitidos, límites de cuerpo, login con token fresco, confirmación de cambios y persistencia de insignias. `npm run test:browser` prueba login obligatorio, navegación, carga progresiva, importación/exportación cifrada, notas, bloqueo, vistas y diseño móvil con respuestas simuladas: no modifica cuentas reales. Las verificaciones adicionales de Whakoom deben comprobarse en el alojamiento definitivo antes de anunciar la conexión como disponible.
