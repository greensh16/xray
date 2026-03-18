"""
Fixture for testing inline suppression comments.
Every bad pattern here is accompanied by a suppression comment so
xray should emit zero diagnostics for this file.
"""
import xarray as xr
import numpy as np
import dask.array as da

# XR001 suppressed on this line
ds = xr.open_dataset("era5.nc")  # xray: disable=XR001

# XR002 suppressed on this line
arr = ds["u10"].values  # xray: disable=XR002

# XR003 suppressed on this line
for t in ds.time:  # xray: disable=XR003
    print(t)

# XR004 suppressed on this line
point = ds.sel(lat=45.0)  # xray: disable=XR004

# NP003 suppressed on this line
grid = np.zeros((512, 512))  # xray: disable=NP003
