# Cambios

## 2.0.0

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
