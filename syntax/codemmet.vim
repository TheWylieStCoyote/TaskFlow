" Vim syntax file
" Language: codemmet source files (.codemmet, file mode)
" The format is line-oriented: `#lang`/`#repeat` directives, `# ` comments,
" blank lines, and one abbreviation per line. Keep this modest.

if exists("b:current_syntax")
  finish
endif

" `#word` that is not a known directive is a hard error in the format.
" Known directives are defined after, so they win at the same position.
syn match codemmetDirectiveError "^\s*#\S\+"
syn match codemmetDirective "^\s*#\%(lang\|repeat\)\>" nextgroup=codemmetDirectiveValue skipwhite
syn match codemmetDirectiveValue ".*" contained

" `#` followed by whitespace (or alone) is a comment.
syn match codemmetComment "^\s*#\%(\s.*\)\=$"

" Abbreviation-line pieces: composition operators and quoted arguments.
syn match codemmetOperator "[>+*^@:,()\[\]]"
syn region codemmetString start=+"+ skip=+\\"+ end=+"+ oneline
syn region codemmetString start=+'+ skip=+\\'+ end=+'+ oneline

hi def link codemmetDirectiveError Error
hi def link codemmetDirective PreProc
hi def link codemmetDirectiveValue Constant
hi def link codemmetComment Comment
hi def link codemmetOperator Operator
hi def link codemmetString String

let b:current_syntax = "codemmet"
