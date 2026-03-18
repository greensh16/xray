"""
Fixture for testing file-level suppression.
The disable-file comment suppresses XR001 for the entire file.
"""
# xray: disable-file=XR001
import xarray as xr

# Both of these would normally fire XR001 — suppressed file-wide
ds1 = xr.open_dataset("era5.nc")
ds2 = xr.open_mfdataset("era5_*.nc")
