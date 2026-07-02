# rurix-godot

`rurix-godot` is the C ABI bridge for the Godot D3D12 Forward+ acceleration
experiment.

The bridge is intentionally opt-in and conservative:

- only D3D12 + Forward+ capabilities are accepted;
- Godot owns the native D3D12 device, queue, resources, and fallback renderer;
- Rurix records resources and pass intent behind an opaque session handle;
- unsupported pass IDs return `RXGD_STATUS_FALLBACK`, allowing Godot to keep its
  original pass.

Concrete accelerated passes are added behind this ABI as the benchmark ladder
proves performance and image-quality wins.
