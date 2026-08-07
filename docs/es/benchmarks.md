# Metodología de benchmarks

Los benchmarks responden a una pregunta estrecha: cómo se comporta `outcome:failure AND user:ana` con el índice frente al evaluador lineal de referencia sobre datos sintéticos deterministas.

## Método

- Build release con LTO, una unidad de generación y Rust 1.97.1 estable.
- Semilla `42`, escenario mixto y 1k, 10k, 100k y 1M de eventos.
- 100, 50, 20 y 3 iteraciones de consulta respectivamente.
- Construcción del índice medida por separado.
- `black_box` conserva el trabajo y ambos evaluadores deben devolver el mismo número de resultados.
- Cada JSON bruto contiene fecha UTC, sistema, arquitectura, procesador y número de procesadores lógicos.

## Resultados del 07/08/2026

| Eventos | Construcción | Media indexada | Media lineal | Coincidencias | Relación observada |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1.000 | 10,93 ms | 24,91 µs | 322,22 µs | 26 | 12,93× |
| 10.000 | 99,94 ms | 241,73 µs | 3,81 ms | 221 | 15,78× |
| 100.000 | 1,17 s | 3,39 ms | 35,37 ms | 2.149 | 10,44× |
| 1.000.000 | 9,76 s | 41,85 ms | 10,22 s | 20.861 | 244,16× |

Equipo: Windows x86-64, Intel64 Family 6 Model 94 Stepping 3, 4 procesadores lógicos. Son resultados de esta ejecución, no una promesa universal. Caché, asignador, energía, distribución de datos y las tres iteraciones en 1M pueden alterar materialmente el resultado.

Criterion cubre comparaciones estadísticas de 1k a 100k. El comando CLI registra manualmente escalas mayores para no introducir métricas inestables en runners compartidos.

