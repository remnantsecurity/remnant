# Typosquat signals

This evaluation area studies reproducible signals that may help identify npm
package names for typosquatting review. A signal records an observable
relationship; it does not independently establish publisher intent or classify
a package as malicious.

Each signal is evaluated separately so its inputs, transformations, resource
limits, recall, and legitimate-package noise remain explicit. Combining signals
or promoting one into Remnant policy requires separate evidence and approval.

Current signals:

- [`package-name-similarity/`](package-name-similarity/) — restricted
  Damerau-Levenshtein distance-one comparison over complete validated package
  names.
