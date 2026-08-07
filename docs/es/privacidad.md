# Privacidad y modelo de amenaza

TraceForge es una aplicación estática. Los bytes importados pasan directamente a la memoria WebAssembly y nunca se envían mediante `fetch`, XHR, WebSocket, subida del service worker ni SDK de analítica. No existen cuentas, cookies ni logs de servidor propios de TraceForge.

La frontera web rechaza archivos de más de 50 MB y datasets de más de 100.000 eventos válidos. Las filas inválidas se truncan a 240 caracteres en los informes. El service worker solo almacena recursos GET necesarios para la aplicación.

Los ejemplos usan rangos IP de documentación e identidades inventadas. No deben publicarse capturas con incidentes reales. Extensiones del navegador, proveedor de hosting y dispositivo local quedan fuera de la frontera de confianza de la aplicación.

