; Zed's documented injection names are @injection.content and
; injection.language. Do not also capture @content: Zed rejects injection
; queries that contain both names in the same pattern.
((text) @injection.content
  (#set! injection.language "yaml")
  (#set! injection.combined))

; The outer YAML injection correctly sees inline templates as block scalars.
; Overlay only text chunks that start like generated YAML inside deeply indented
; function-go-templating block scalars. Chunks may begin with blank lines,
; comments, or a document marker before the first YAML key. Keeping this narrow
; avoids injecting the same ordinary YAML text range twice, which duplicates Zed
; outline entries. Tree-sitter query regex support is limited here, so the
; repeated whitespace classes intentionally spell out a ten-column minimum
; instead of using {10,}.
((text) @injection.content
  (#match? @injection.content "^\n([ \t]*(#.*|---)?\n)*[ \t][ \t][ \t][ \t][ \t][ \t][ \t][ \t][ \t][ \t][ \t]*(-[ \t]+)?[A-Za-z0-9_./-]+:")
  (#set! injection.language "yaml"))
