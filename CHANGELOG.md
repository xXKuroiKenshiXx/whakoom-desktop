# Cambios

## Cambios pendientes de publicación

- Lecturas muestra sólo leídos y tomos en lectura; se añade el estado local Leyendo.
- Cifrado de biblioteca, preferencias y páginas privadas, migración de datos anteriores y respaldos cifrados portables. Se documenta qué datos se protegen y los límites del cifrado.
- Insignias de 1000, 2000 y 4000 tomos; brillo según dificultad, holografía y zoom al pasar el mouse. Pro usa un indicador violeta más grande.
- Los perfiles permiten consultar Seguidos y Seguidores cuando son visibles en Whakoom.
- Tutorial con estrellas doradas y violetas centradas y pasos propios para Listado Manga e insignias; explica valoraciones, personas y estadísticas; se puede repetir desde Ajustes. Novedades usa filtros Todas/Mis series y las acciones de colección centran su contenido.
- Biblioteca web Astro separada, preparada para Netlify con almacenamiento local cifrado y respaldos compatibles. No incluye login ni sincronización online con Whakoom.

## 3.2.0

- Catálogo incorpora Crear ficha; las series y los tomos permiten Modificar ficha y Sugerir un cambio. Las series también ofrecen Añadir tomos. Usan los formularios oficiales de Whakoom, conservando validaciones, permisos y revisión de sugerencias.
- Windows integra los formularios con la sesión conectada mediante cookies HttpOnly del mismo dominio, sin contraseñas en JavaScript. Linux abre el formulario en el navegador predeterminado, con su propia sesión. Al volver en Windows se consulta de nuevo la ficha.
- Cuenta y editor de perfil: tipografía más grande, pestañas de 44 px y botones amplios para cambiar el perfil y abrir Insignias, con textos que se adaptan a ventanas pequeñas.

## 3.1.0

- Insignias: 18 logros con medallas vectoriales, niveles de bronce, plata, oro y prisma, progreso y desbloqueos persistentes por cuenta. Se añaden diez logros de lecturas, colección, series completas, notas, relecturas, organización y valoraciones.
- Aviso de desbloqueo con destellos, sonido breve y acceso a Insignias. Los logros anteriores no vuelven a celebrarse al abrir la app. El sonido se desactiva en Ajustes y los efectos respetan la opción de animaciones.
- Marca Pro junto al nombre de la cuenta y de otras personas cuando Whakoom la identifica en el perfil, resultados o comentarios. Se reutilizan las consultas existentes, sin atribuir Pro a usuarios por sus medallas locales.
- Las cuatro acciones de una serie tienen el mismo ancho y alto, incluido «Completar colección».
- Menos solicitudes: las portadas sin caché se descargan una sola vez en la calidad elegida, las tarjetas reutilizan valoraciones guardadas y Listado Manga consulta portadas sólo de resultados visibles, sin repetir la primera búsqueda.
- Las conexiones de catálogo, cuenta y portadas comparten un límite de dos solicitudes simultáneas y una separación mínima entre solicitudes. Ante HTTP 429 respetan `Retry-After` y conservan los cambios pendientes y los datos guardados durante la pausa.
- Listado Manga muestra la primera coincidencia sin esperar todas las variantes; las búsquedas amplias y las portadas se incorporan progresivamente en segundo plano.
- Las acciones de series mantienen un ancho uniforme y las colecciones completas muestran «Serie completada» en lugar de «Te faltan 0 tomos».

- Mi biblioteca abre en Tomos faltantes con el tomo de menor numeración que no tenés de cada colección, incluso si compraste tomos posteriores. Los datos guardados aparecen al entrar; se actualizan sólo las ediciones desconocidas o con más de tres horas, en tandas de hasta tres consultas. Recargar fuerza la actualización.
- Buscador de biblioteca más amplio y alto, con lupa y modos Tomos faltantes, Series y Tomos dentro de la misma barra; se adapta a ventanas estrechas.
- El gasto total de «Tu colección en cifras» queda integrado en «Tus ritmos de compra y lectura», conservando la separación por monedas.

- Deseados reúne tomos y series con filtros; se elimina la sección redundante Favoritos de cómics. Personas reúne Seguidos, Seguidores y contactos favoritos locales con corazón y carrusel.
- Lecturas permite alternar lista y portadas. Listado Manga combina variantes del título, ordena coincidencias y carga portadas de resultados en segundo plano.
- Valoración personal compacta bajo la comunidad; Comprar junto a Leído, acceso a Listado Manga debajo de la portada y selector de tiendas con iconos de marca.
- Los enlaces Amazon que devolvían JSON se resuelven antes de abrir el navegador; se verifica el dominio y se ofrece búsqueda por título si falla el enlace directo.
- Progreso de propiedad y tomos faltantes en tarjetas de colecciones completas; panel de serie mayor, botones separados y Opiniones debajo de la acción de colección.
- Ejemplos de calidad ampliados con detalle de nitidez y filtro de novedades de mis series más visible.
- Las tarjetas de biblioteca siguen siendo interactivas mientras se consulta la colección; la carga ya no bloquea abrir una serie o tomo.
- Cuenta incorpora Insignias locales con progreso para lecturas, tomos, colecciones, deseados y valoraciones. Se calculan sin conexión.

- Cuenta: navegación superior con iconos; Perfil muestra sólo el resumen de tu cuenta. Foto, nombre público y biografía se editan en un diálogo separado después de los controles de desconexión.
- Mi biblioteca incorpora Tomos faltantes para las ediciones que coleccionás. Sugiere el siguiente tomo publicado y permite consultar los huecos anteriores.
- Todas las ediciones tienen filtros Todos, Tengo y Faltan con contadores y estado visible por tomo. Se abre la edición exacta desde la biblioteca y las sugerencias.
- Carga automática de todas las páginas en segundo plano, sin bloquear la navegación. La última colección completa permanece en caché si la consulta se interrumpe.
- Lectura compatible con los indicadores actuales de propiedad de Whakoom y nuevos controles traducidos a los cinco idiomas.

## 3.0.0

- Usuarios como sección propia del catálogo, con iconos nuevos para catálogo y listas.
- Aviso de actualizaciones, descarga desde GitHub y verificación SHA-256 antes de instalar y reiniciar en Windows o desde AppImage.
- Deseados reúne Buscados, todas las páginas del servicio y del perfil; contrasta deseos conocidos con fichas actuales. Favoritos de series actualizan su estado local inmediatamente.
- Las series desplazan el encabezado y los tomos juntos; se conserva la virtualización de tarjetas y se corrige el recorte al bajar en ventanas pequeñas.
- Listado Manga con navegación interna, búsqueda real y acceso desde fichas de series y tomos; página original integrada en Windows.
- Comprar con tiendas de Whakoom, Amazon cuando está disponible y búsqueda dinámica de título/tomo en Mercado Libre.
- Opinión pública con botón dorado más amplio; fechas alineadas, campos mayores y moneda por importe pagado. Estadísticas y CSV conservan monedas diferentes sin sumarlas.
- Tarjetas de almacenamiento simétricas en dos columnas, apiladas en ventanas pequeñas, e historial con botón más ancho.
- Controles nuevos traducidos a inglés, portugués, ruso y chino.

## 2.0.5

- Deseados combina la página Buscados y el servicio paginado; reconoce el final numérico de la respuesta para evitar descartar la actualización completa.
- Perfiles con Actividad, Comicteca, Buscados y Listas, paginación y búsqueda de usuarios desde Catálogo.
- Actividad de seguidores y consultas sociales en tandas de hasta tres perfiles.
- Carrusel siempre habilitado con las animaciones, tarjetas mayores y botones de Amigos/Seguidores más visibles.
- Nuevas altas de tomos y series registran el día en la fecha de compra, manteniendo intactas las fechas históricas desconocidas.
- Objetivo de lectura al final de Estadísticas, controles de año alineados, Ajustes con tarjetas de ancho completo e icono de recarga más claro.
- Paquetes Windows y Linux con el número completo 2.0.5.

## 2.0.0

- Iconos propios en las siete opciones de cuenta, con selección destacada.
- Las portadas en las tarjetas abren la ficha o serie; el visor ampliado sólo se abre desde el interior de la ficha.

- Visor de portada con zoom, arrastre, Escape y descarga independiente en alta calidad.
- Catálogo reorganizado: navegación, búsqueda e historial separados; iconos accesibles para refrescar y alternar vista.
- Deseados unifica la lista antes duplicada en Buscados y se actualiza al entrar.
- Opiniones públicas de tomos y series: consulta de la opinión actual, edición, envío persistente y confirmación del servidor. Una valoración nueva actualiza los reintentos de la opinión.
- Estadísticas anuales de compras y lecturas, gráfico mensual, fecha de compra en las fichas y exportación CSV. Lecturas originales cuando la cuenta concede acceso.
- Comunidad de ayuda pública con categorías, publicaciones y respuestas; reportes e ideas mediante formularios oficiales en una sesión separada.
- ISBN y cantidad de propietarios en fichas; Leído activo en verde y reacciones locales compactas al pie del comentario.
- Ejemplos de calidad de portadas en Almacenamiento y en el asistente; carruseles adaptados al ancho de ventana, incluso con un contacto.
- Textos nuevos traducidos a inglés, portugués, ruso y chino. No se cambian las dependencias ni la ubicación de datos existentes.

## 1.0.0

Primera versión; recibió las iteraciones siguientes antes de pasar a 2.0.0 por solicitud del usuario.

- Inicio guiado con fondo atenuado: elegir caché y calidad, avanzar y volver; alta calidad por defecto. Se puede repetir desde Almacenamiento.
- Carruseles infinitos de Amigos y Seguidores, también con pocos contactos; pausa al interactuar y animaciones desactivables.
- Un único botón con icono alterna portadas y lista. Barra lateral animada con Favoritos, Lecturas, Deseados y Amigos en ese orden.
- Título y gasto de estadísticas en dorado, con contraste para ambos temas.
- La sincronización de colección no bloquea la navegación desde Notificaciones; los errores tardíos liberan el estado de actualización sin cambiar la sección abierta.

- Seguidos y seguidores separados en Amigos, con perfiles internos y caché por cuenta.
- Hover conservado hasta 18 px fuera de la portada; salida gradual en 220 ms.
- Ajustes con secciones General, Almacenamiento y Respaldo; límite explícito de imágenes separado del espacio y la resolución.
- Optimización sin pérdida en segundo plano para imágenes guardadas, sin aumentar archivos ni restaurar caché borrada; fotos personales usan el menor PNG/WebP sin pérdida.

- Catálogo integrado con Explorar, Buscados, historial de búsquedas y visitas, y Listas propias/favoritas/creación online.
- Portadas progresivas con colas independientes para miniaturas y resolución final; respaldo de símbolos matemáticos y CJK.
- Español, inglés, portugués, ruso y chino; carrusel opcional de amigos y ajustes adaptables.
- Relecturas, cola de lectura, ubicación, conservación, filtros y álbum local de firmas incluido en el respaldo.
- El empaquetado Linux reemplaza el ejecutable al reutilizar la misma AppDir, evitando publicar un binario anterior.

- Calendario persistente al navegar por meses, con selector de mes y año.
- Corazón de deseados relleno y rojo, sinopsis con color, mayor separación de la portada y emojis dentro de notas.
- Marco iridiscente sutil y animado, desactivable junto con las demás animaciones.
- Quitar colecciones completas conservando las notas y lecturas.
- Envíos de series con hasta tres solicitudes simultáneas y verificación final.
- Sesión Linux en Secret Service, guardado atómico, AppImage, instalador Windows y portable.

- Versión 1.0 solicitada explícitamente; las próximas iteraciones reemplazan este mismo ejecutable.
- Hover en 150 ms, zoom de 9,5%, inclinación instantánea según el mouse y reflejos más brillantes.
- Caché configurable: activación, espacio en MB/GB, cantidad de miniaturas, resolución del CDN y límite de imágenes en RAM; limpieza de las menos usadas.
- Series completas: comprobación de la colección real, recuperación de cambios parciales en tandas de 12 y reintentos automáticos. Editar un tomo durante una tanda conserva la intención más reciente.
- Corazones y dislikes locales por cuenta y ficha: los destacados reaparecen arriba aunque la opinión esté en otra página; los dislikes bajan al final. Incluidos en los respaldos.
- Botones de colección con iconos, sinopsis de mayor tamaño, votos dorados con contraste en ambos temas y tarjetas de lectura/notas con calendario y selector de emojis.

- Cuenta separada de Ajustes y botón completo de usuario, sin selección de texto.
- Formularios reales de perfil, avatar, seguridad, notificaciones, región y privacidad; consulta y desbloqueo de usuarios.
- Estado de suscripción, canje de códigos y cancelación de renovación; gestión de pagos por el flujo oficial.
- Opiniones de tomos y ediciones con paginación, autor, avatar, fecha y estrellas; cantidad de votos en las fichas.
- Fichas consultadas automáticamente al abrirlas y volver a entrar; consulta manual eliminada.
- Campana de actividad de amigos, contador y consulta periódica mientras la app está abierta.
- Estadísticas reorganizadas con tarjetas centradas, gráficos adaptables y controles de lectura grandes.
- Guardado de cuenta accesible en formularios largos y limpieza de distribuciones anteriores.

- Proyecto y ejecutable renombrados a Whakoom Desktop; datos de instalaciones previas conservados.
- Colección y deseados se actualizan automáticamente al entrar a Mi biblioteca.
- Cola persistente por cuenta para colección, series completas, favoritos, lectura, valoración y notas habilitadas; envío automático, reintentos y errores visibles.
- Reconciliación de eliminaciones online, protección de pendientes y migración de cambios locales anteriores.
- Avatar real, perfiles internos y Amigos con actividad pública reciente.
- Valoraciones de comunidad doradas y personales violetas de cuatro puntas, incluidas valoraciones de series.
- Holografía, zoom e inclinación de portadas desde 150 ms de hover, con opción para desactivar animaciones.
- Ajustes y perfil reorganizados con iconos y tarjetas.
- Limpieza de archivos temporales y versiones anteriores; documentación y CI de Windows.
