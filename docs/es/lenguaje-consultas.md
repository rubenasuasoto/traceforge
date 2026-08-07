# Lenguaje de consultas

## Gramática

```ebnf
consulta    = expresion_or ;
expresion_or = expresion_and, { "OR", expresion_and } ;
expresion_and = expresion_unaria, { ["AND"], expresion_unaria } ;
expresion_unaria = ["NOT"], (termino | "(", expresion_or, ")") ;
termino     = [campo, ":"], (palabra | comillas | prefijo | rango_temporal) ;
prefijo     = palabra, "*" ;
rango_temporal = "[", rfc3339, "TO", rfc3339, "]" ;
```

La precedencia es `NOT`, después `AND` —incluida la adyacencia implícita— y finalmente `OR`. Las palabras clave no distinguen mayúsculas. Los valores se normalizan para buscar, pero los eventos originales no se modifican.

## Campos indexados

`id`, `source`, `type`/`event_type`, `user`, `host`, `ip`/`source_ip`, `outcome`/`result`, `severity` y texto de mensaje. Un término libre busca tokens del mensaje. Una frase entre comillas intersecta primero posting lists y verifica después la frase solo sobre los candidatos.

## Errores y rangos

El parser devuelve posición y motivo para entrada vacía, valores ausentes, comillas o grupos abiertos, rangos inválidos y tokens inesperados. No degrada silenciosamente a un escaneo amplio. Los rangos son inclusivos, admiten RFC 3339 y solo se aplican a `timestamp`/`time`.

