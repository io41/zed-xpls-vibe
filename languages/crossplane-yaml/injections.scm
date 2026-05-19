; Zed's documented injection names are @injection.content and
; injection.language. Do not also capture @content: Zed rejects injection
; queries that contain both names in the same pattern.
((text) @injection.content
  (#set! injection.language "yaml")
  (#set! injection.combined))

; The outer YAML injection correctly sees inline templates as block scalars.
; Overlay YAML-looking text chunks as standalone YAML fragments as well. This
; gives generated YAML inside function-go-templating block scalars its own YAML
; parse instead of inheriting the outer block-scalar highlight.
((text) @injection.content
  (#match? @injection.content "(^|\n)[ \t]*[A-Za-z0-9_./-]+:")
  (#set! injection.language "yaml"))
