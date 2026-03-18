import numpy as np
import netCDF4
import zarr
import h5py
import xarray as xr

arr = np.random.rand(1024, 1024).astype(np.float32)

# IO001: np.save for a large array — uncompressed, single chunk
np.save("wind_u.npy", arr)
np.save("wind_v.npy", arr)

# IO002: netCDF4.Dataset opened directly — bypasses xarray
nc = netCDF4.Dataset("era5.nc", "r")
u10 = nc.variables["u10"][:]
nc.close()

# IO003: zarr.open without chunks= — entire array as one chunk
store = zarr.open("wind.zarr", mode="w", shape=(8760, 721, 1440), dtype="f4")

# IO003: zarr.open_array without chunks
z_arr = zarr.open_array("pressure.zarr", mode="r")

# IO003: This is fine — has chunks
z_ok = zarr.open("wind_ok.zarr", mode="w", shape=(8760, 721, 1440),
                 chunks=(24, 181, 360), dtype="f4")

# IO004: netCDF4 variable subscripted inside a loop — repeated disk reads
nc2 = netCDF4.Dataset("barra2.nc", "r")
temp = nc2.variables["temp"]
monthly_means = []
for i in range(12):
    monthly_means.append(temp[i].mean())   # each temp[i] hits disk
nc2.close()

# IO005: h5py.File without swmr=True — stale reads in parallel HPC scenarios
f = h5py.File("data.h5", "r")

# IO006: engine="scipy" — eager load, no chunking/lazy evaluation
ds_scipy = xr.open_dataset("data.nc", chunks="auto", engine="scipy")
