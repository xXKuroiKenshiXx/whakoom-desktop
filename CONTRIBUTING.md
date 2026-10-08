# Contribuir

Compilá y ejecutá las comprobaciones del README en Windows y Linux antes de enviar cambios. Usá fixtures mínimos con usuarios e identificadores ficticios para las pruebas de sesión y sincronización. Las pruebas de red manuales quedan fuera de la suite normal.

El conector conserva los cambios hasta que el servidor confirma la operación. Ante cambios del servicio, verificá la forma actual de sus respuestas y agregá una prueba del contrato afectado. No aceptes una respuesta genérica como éxito sin comprobar el estado correspondiente.

No incluyas cookies, contraseñas, `session.dpapi`, bibliotecas, respaldos, HTML autenticado completo ni capturas de perfiles personales en commits o issues. Conservá los diagnósticos fuera del proyecto. La `.gitignore` excluye datos locales y artefactos de compilación; revisá también manualmente los archivos que publiques.

La interfaz principal debe permanecer nativa. Reservá el WebView para verificar el acceso y cerralo al completar la conexión. Las animaciones deben respetar Ajustes y detener los repintados cuando no se usan. Mantené acotadas las colas, imágenes y consultas por ficha.

La versión permanece en 1.0.0 hasta un pedido explícito de cambio. Las iteraciones reemplazan los archivos actuales de `dist`; no deben crear carpetas de historial ni copias numeradas. `tools/package.ps1` recompila y sobrescribe esos mismos paquetes sin aumentar la versión.

Las opciones de Cuenta usan sus formularios reales con verificación antifalsificación. Los formularios y contraseñas permanecen en memoria, separados de la biblioteca exportable. Una respuesta de otro usuario o sección no debe sobrescribir el formulario abierto.
