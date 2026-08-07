# Query language

## Grammar

```ebnf
query       = or_expression ;
or_expression = and_expression, { "OR", and_expression } ;
and_expression = unary_expression, { ["AND"], unary_expression } ;
unary_expression = ["NOT"], (term | "(", or_expression, ")") ;
term        = [field, ":"], (word | quoted | prefix | time_range) ;
prefix      = word, "*" ;
time_range  = "[", rfc3339, "TO", rfc3339, "]" ;
```

Precedence is `NOT`, then `AND` (including implicit adjacency), then `OR`. Keywords are case-insensitive. Values are normalized to lowercase for matching; the original events are preserved.

## Indexed fields

`id`, `source`, `type`/`event_type`, `user`, `host`, `ip`/`source_ip`, `outcome`/`result`, `severity` and message text. Plain terms search message tokens. Quoted multi-token text intersects token posting lists and then verifies the phrase against candidate messages.

## Errors

The parser returns a character position and specific reason for empty input, missing values, unclosed quotes/groups, invalid ranges or unexpected tokens. Invalid queries never fall back to a broad scan.

## Temporal ranges

Only `timestamp`/`time` accepts range syntax. Endpoints are RFC 3339 and inclusive. Binary searches locate the lower and upper bounds, after which matching event IDs are normalized to posting-list order.

