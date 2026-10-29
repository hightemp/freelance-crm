// Parts shared by the invoice templates. Data comes as JSON; user text is
// inserted with #expr, so it is never parsed as markup.
#let d = json(bytes(sys.inputs.data))
#let l = d.labels

#let money(value) = [#value #d.currency]

#let party(title, p) = block[
  #text(weight: "bold")[#title] \
  #p.name \
  #for line in p.lines [#line.label: #line.value \ ]
]

#let items-table() = table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (center, left, right, center, right, right),
  stroke: 0.5pt + luma(180),
  inset: 5pt,
  [#l.n], [#l.description], [#l.quantity], [#l.unit], [#l.price], [#l.amount],
  ..d.items.map(i => ([#i.n], [#i.description], [#i.quantity], [#i.unit], [#i.price], [#i.amount])).flatten(),
)

#let totals() = align(right)[
  #l.subtotal: #money(d.subtotal) \
  #if d.discount != none [#l.discount: #money(d.discount) \ ]
  #if d.tax != none [#d.tax.label: #money(d.tax.value) \ ]
  #text(weight: "bold")[#l.total: #money(d.total)]
]

#let signature() = block[
  #l.signature:
  #if d.signature_file != none {
    box(image(d.signature_file, height: 1.5cm))
  } else {
    box(width: 5cm, stroke: (bottom: 0.5pt))[]
  }
  #d.seller.name
  #if d.stamp_file != none { place(dx: 1cm, dy: -2cm, image(d.stamp_file, height: 3cm)) }
]
