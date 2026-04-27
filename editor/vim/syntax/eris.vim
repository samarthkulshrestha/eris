" Vim syntax file
" Language: Eris
" Maintainer: Eris project
"
" To enable: add the parent directory to runtimepath, e.g.
"   set runtimepath^=/path/to/eris/editor/vim

if exists("b:current_syntax")
  finish
endif

" --- Comments -----------------------------------------------------------
" Doc comments (--- ...) and line comments (-- ...). The doc-comment
" pattern is defined first so it wins on '---' prefixes via Vim's
" longer-match-wins rule. The line-comment pattern explicitly excludes
" a third dash and also handles the bare '--' case.
syn match erisDocComment    "---.*$"
syn match erisLineComment   "--$"
syn match erisLineComment   "--[^-].*$"

" --- Keywords -----------------------------------------------------------
syn keyword erisKeyword     if else return while break continue mut
syn keyword erisType        int float char bool str arr
syn keyword erisBoolean     true false

" Built-in functions provided by the compiler.
syn keyword erisBuiltin     print_int print_str print_char print_bool println read_int

" --- Literals -----------------------------------------------------------
" Floats first so they win against the integer pattern on the leading digits.
syn match erisFloat         "\<\d\+\.\d\+\>"
syn match erisInt           "\<\d\+\>"

syn match erisChar          "'\\\?.'"
syn match erisChar          "'\\\\'"
syn region erisString       start=+"+ end=+"+ skip=+\\"+

" --- Operators ----------------------------------------------------------
syn match erisOperator      ":="
syn match erisOperator      "[=!<>]="
syn match erisOperator      "&&\|||"
syn match erisOperator      "[+\-*/%=<>!]"

" --- Function definitions / call sites ----------------------------------
" Heuristic: an identifier immediately followed by '(' is a function name.
syn match erisFunction      "\<\h\w*\ze\s*("

" --- Highlight groups ---------------------------------------------------
hi def link erisLineComment  Comment
hi def link erisDocComment   SpecialComment
hi def link erisKeyword      Keyword
hi def link erisType         Type
hi def link erisBoolean      Boolean
hi def link erisBuiltin      Function
hi def link erisInt          Number
hi def link erisFloat        Float
hi def link erisString       String
hi def link erisChar         Character
hi def link erisOperator     Operator
hi def link erisFunction     Function

let b:current_syntax = "eris"
