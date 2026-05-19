" Vim syntax file
" Language: Eris
" Maintainer: Eris project
"
" To enable: add the parent directory to runtimepath, e.g.
"   set runtimepath^=/path/to/eris/editor/vim

if exists("b:current_syntax")
  finish
endif

" Pattern definition order matters in this file: when multiple `syn match`
" items can start at the same position, the one defined LAST wins. The
" comment patterns are intentionally placed at the bottom so that they
" beat the single-char `-` arm of the operator pattern; otherwise the
" `--` of a comment renders as two operators.

" --- Keywords -----------------------------------------------------------
syn keyword erisKeyword     if else return while break continue for in mut
syn keyword erisType        int float char bool str arr
syn keyword erisBoolean     true false

" Built-in functions provided by the compiler.
syn keyword erisBuiltin     print_int print_str print_char print_bool println read_int

" --- Literals -----------------------------------------------------------
syn match erisFloat         "\<\d\+\.\d\+\>"
syn match erisInt           "\<\d\+\>"

syn match erisChar          "'\\\?.'"
syn match erisChar          "'\\\\'"
syn region erisString       start=+"+ end=+"+ skip=+\\"+

" --- Operators ----------------------------------------------------------
syn match erisOperator      ":="
syn match erisOperator      "\.\."
syn match erisOperator      "[=!<>]="
syn match erisOperator      "&&\|||"
syn match erisOperator      "[+\-*/%=<>!]"

" --- Function names -----------------------------------------------------
" Heuristic: an identifier immediately followed by '(' is a function name.
syn match erisFunction      "\<\h\w*\ze\s*("

" --- Comments (LAST: must win the priority battle against `-` operator) -
syn match erisDocComment    "---.*$"
syn match erisLineComment   "--$"
syn match erisLineComment   "--[^-].*$"

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
