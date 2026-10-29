#import "common.typ": *
#set page(paper: "a4", margin: 2cm)
#set text(font: "Noto Sans", size: 10pt, lang: d.lang)

#if d.logo_file != none { image(d.logo_file, height: 1.5cm) }
#text(size: 14pt, weight: "bold")[#d.title]
#v(4mm)
#party(l.seller, d.seller)
#party(l.buyer, d.buyer)
#v(4mm)
#items-table()
#totals()
#v(2mm)
#if d.vat_note != none [#d.vat_note \ ]
#if d.amount_in_words != none [#l.in_words: #d.amount_in_words \ ]
#if d.due_date != none [#l.due: #d.due_date \ ]
#if d.notes != "" [#l.notes: #d.notes]
#v(1cm)
#signature()
