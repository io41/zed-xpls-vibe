; Identifiers

[
  (field)
  (field_identifier)
] @property

(variable) @variable

; Function calls

(function_call
  function: (identifier) @function)

(method_call
  method: (selector_expression
    field: (field_identifier) @function))

; Operators

"|" @operator
":=" @operator

; Builtin, Sprig, and Crossplane go-templating helpers

((identifier) @function.builtin
  (#match? @function.builtin "^(and|call|html|index|slice|js|len|not|or|print|printf|println|urlquery|eq|ne|lt|le|gt|ge|default|dig|empty|fail|quote|randomChoice|toJson|toYaml|fromYaml|trim|indent|nindent|b64enc|b64dec|getResourceCondition|getComposedResource|getComposedConnectionDetails|getCompositeResource|getExtraResources|getExtraResourcesFromContext|setResourceNameAnnotation|include)$"))

; YAML document markers emitted by the go-template grammar.
; Keep this highlight-only; do not inject it as YAML content by default.
; Scope this to top-level template children so malformed actions under ERROR
; do not look like YAML document markers.

(template
  [
    (yaml_document_marker)
    (yaml_no_injection_text)
  ] @punctuation.special)

; Delimiters

"." @punctuation.delimiter
"," @punctuation.delimiter

"{{" @punctuation.bracket
"}}" @punctuation.bracket
"{{-" @punctuation.bracket
"-}}" @punctuation.bracket
")" @punctuation.bracket
"(" @punctuation.bracket

; Keywords

"else" @keyword
"if" @keyword
"range" @keyword
"with" @keyword
"end" @keyword
"template" @keyword
"define" @keyword
"block" @keyword

; Literals

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @string.special

[
  (int_literal)
  (float_literal)
  (imaginary_literal)
] @number

[
  (true)
  (false)
  (nil)
] @constant.builtin

(comment) @comment
(ERROR) @error
