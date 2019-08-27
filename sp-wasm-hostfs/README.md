
### gwasm_hostfs_volumes() -> VolumeInfo[]

```yaml
VolumeInfo:
    id: volume id (int)
    mount_point: path in image to mount
    mode: (ro|rw|wo)
```

### gwasm_hostfs_lookup(vol_id, path) -> NodeInfo

```yaml
NodeInfo:
    mode: file_mode
```

### Open 

`gwasm_hostfs_open(vol_id, path, flags, mode) -> int`

### Close 
`gwasm_hostfs_close(fd) -> int`


### Read 

`gwasm_hostfs_read(fd, buf, offset, len, position) -> int`

### Write 

`gwasm_hostfs_write(fd, buf, offset, length, positon) -> int`

### Readdir 

`gwasm_hostfs_readdir(vol_id, path) -> [String]`


