; IO001 — np.save() used to persist large arrays.
; For anything over ~1 MB, prefer Zarr (chunked, compressed, cloud-native)
; or HDF5.  np.save is uncompressed, unchunked, single-file.
(call
  function: [
    (attribute
      attribute: (identifier) @io_npsave
      (#eq? @io_npsave "save")
    )
    (identifier) @io_npsave_bare
    (#eq? @io_npsave_bare "save")
  ]
) @io_npsave_call


; IO002 — netCDF4.Dataset opened directly (bypasses xarray alignment machinery).
; Prefer xr.open_dataset for all coordinate-aligned work.
(call
  function: (attribute
    object: (identifier) @io_nc4_mod
    attribute: (identifier) @io_nc4_class
    (#eq? @io_nc4_mod "netCDF4")
    (#eq? @io_nc4_class "Dataset")
  )
) @io_nc4_dataset_call


; IO003 — zarr.open / zarr.open_array / zarr.open_group called without
; chunks= argument.  Unchunked Zarr defeats the point of Zarr entirely.
(call
  function: [
    (attribute
      attribute: (identifier) @io_zarr_open
      (#match? @io_zarr_open "^open(_array|_group)?$")
    )
    (identifier) @io_zarr_fn
    (#match? @io_zarr_fn "^open(_array|_group)?$")
  ]
  arguments: (argument_list) @io_zarr_args
) @io_zarr_open_call


; IO004 — subscript on a plain identifier.  Rust checks is_inside_for_loop()
; and file.imports.netcdf4 before emitting — catches nc_var[i] reads in loops.
(subscript
  value: (identifier) @io_nc_var
) @io_nc_subscript


; IO005 — h5py.File opened without swmr=True.
; In HPC parallel read scenarios SWMR (Single-Writer Multiple-Reader) mode
; prevents stale reads and potential corruption. Omitting it is safe for
; purely serial access but should be considered for any concurrent workflow.
(call
  function: [
    (attribute
      attribute: (identifier) @io_h5py_file_attr
      (#eq? @io_h5py_file_attr "File")
    )
    (identifier) @io_h5py_file_bare
    (#eq? @io_h5py_file_bare "File")
  ]
) @io_h5py_file_call


; IO006 — xr.open_dataset / xr.open_mfdataset called with engine="scipy".
; The scipy engine loads files eagerly without chunking or lazy evaluation;
; use engine="netcdf4" or engine="zarr" for large HPC datasets.
(call
  function: [
    (identifier) @io_open_scipy_bare
    (#match? @io_open_scipy_bare "^open_(mf)?dataset$")
    (attribute
      attribute: (identifier) @io_open_scipy_attr
      (#match? @io_open_scipy_attr "^open_(mf)?dataset$")
    )
  ]
) @io_open_scipy_call
