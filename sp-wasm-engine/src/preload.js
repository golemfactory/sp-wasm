HOSTFS = {

    mount(opts) {
        const {volid} = opts.opts;
        const root = HOSTFS.createNode(null, "/", 0o777 | 16384, 0);
        root.volid = volid;
        root.tag = '';
        return root;
    },

    createNode(parent, name, mode, dev) {
        if (!FS.isDir(mode) && !FS.isFile(mode) && !FS.isLink(mode)) {
            throw new FS.ErrnoError(22);
        }
        const node = FS.createNode(parent, name, mode);
        if (parent) {
            node.volid = parent.volid;
            node.tag = parent.tag + '/' + name;
        }
        if (FS.isDir(mode)) {
            node.node_ops = HOSTFS.ops_table.dir.node;
            node.stream_ops = HOSTFS.ops_table.dir.stream;
        }
        if (FS.isFile(mode)) {
            node.node_ops = HOSTFS.ops_table.file.node;
            node.stream_ops = HOSTFS.ops_table.file.stream;
        }

        return node;
    }
};
HOSTFS.node_ops = {
    getattr: (node) => {
        //print(`getattr ${node.tag} ${node.volid}`);
        const m = hostfs.lookup(node.volid, node.tag);
        //print(`m=${JSON.stringify(m)}`);
        let attr = {};
        attr.dev = 1;
        attr.ino = node.id;
        attr.mode = node.mode;
        attr.nlink = 1;
        attr.uid = 0;
        attr.gid = 0;
        attr.rdev = node.rdev;
        attr.size = m.size;
        attr.atime = new Date(0);
        attr.mtime = new Date(0);
        attr.ctime = new Date(0);
        attr.blksize = 4096;
        attr.blocks = Math.ceil(attr.size / attr.blksize);


        return attr;
    },

    setattr: () => {
        //print("setattr")
    },

    dir_stat: () => {
        print('dir stat');
    },

    file_stat: () => {
        print('file stat');
    },

    lookup: (parent, name) => {
        try {
            //print(`try_lookup ${parent.volid}:${parent.tag}/${name}`);
            const node_info = hostfs.lookup(parent.volid, `${parent.tag}/${name}`);
            let mcode = 0;

            if (node_info.type === 'f') {
                mcode |= 32768;
            } else if (node_info.type === 'd') {
                mcode |= 16384 | 0o111;
            }

            if (node_info.mode == 'ro') {
                mcode |= 0o444;
            } else if (node_info.mode == 'rw') {
                mcode |= 0o666;
            }

            //print(`${parent.volid}:${parent.tag}/${name} ${JSON.stringify(node_info)}`);

            return HOSTFS.createNode(parent, name, mcode, 0);
        } catch (e) {
            throw new FS.ErrnoError(2);
        }
    },

    mknod: (parent, name, mode, dev) => {
        //print(`mknode ${name} ${mode} ${dev}`);
        if (FS.isDir(mode)) {
            const node_info = hostfs.mkdir(parent.volid, `${parent.tag}/${name}`);
            let mcode = 0;

            if (node_info.type === 'f') {
                mcode |= 32768;
            } else if (node_info.type === 'd') {
                mcode |= 16384 | 0o111;
            }

            if (node_info.mode == 'ro') {
                mcode |= 0o444;
            } else if (node_info.mode == 'rw') {
                mcode |= 0o666;
            }
            return HOSTFS.createNode(parent, name, mcode, 0);
        }
        return HOSTFS.createNode(parent, name, mode, dev);
    },

    //rename: () => {},
    //unlink: () => {},
    //rmdir: () => {},

    readdir: (node) => {
        return hostfs.readdir(node.volid, node.tag);
    },

    //symlink: () => {}
};
HOSTFS.stream_ops = {
    llseek: (stream, offset, whence) => {
        // print(`llseek(${stream.node.tag}, ${offset}, ${whence})`);
        let position = offset;
        if (whence == 1) {
            position += stream.position;
        } else if (whence == 2) {
            if (FS.isFile(stream.node.mode)) {
                position += stream.node.usedBytes;
            }
        }

        return position;
    },
    close: stream => {
        const {host_fd} = stream;

        hostfs.close(host_fd);

    },
    open: stream => {
        //print('open in');
        const {volid, tag} = stream.node;
        const {flags} = stream;

        const MODES = ['ro', 'wo', 'rw'];

        let host_fd = hostfs.open(volid, tag, MODES[flags & 3], (flags & 64) == 64);

        stream.host_fd = host_fd;

    },
    read: (stream, buffer, offset, length, pos) => {
        if (length == 0) {
            return 0;
        }

        pos = pos || stream.position;
        //print(`read: ${stream.host_fd}, offset=${offset}, len=${length}, pos=${pos}`);
        try {
            const len = hostfs.read(stream.host_fd, buffer, offset, length, pos);
            /*if (len > 0) {
                stream.position = pos + len;
            }*/
            //print(`read: ${len} bytes`);
            return len;
        } catch (e) {
            print('err');
            return 0;
        }
    },
    write: (stream, buffer, offset, length, pos) => {
        if (length == 0) {
            return 0;
        }
        pos = pos || stream.position;
        //print(`write: ${stream.host_fd}, ${buffer.buffer.length}, ${offset}, ${length}, ${pos}`);
        try {
            const len = hostfs.write(stream.host_fd, buffer, offset, length, pos);
            /*if (len > 0) {
                stream.position = pos + len;
            }*/
            //print(`write: ${len}`);
            return len;
        } catch (e) {
            print('err');
            return 0;
        }
    },

    getdents: () => {
        print('getdents');
    }
};

HOSTFS.ops_table = {
    dir: {
        node: {
            getattr: HOSTFS.node_ops.getattr,
            setattr: HOSTFS.node_ops.setattr,
            lookup: HOSTFS.node_ops.lookup,
            mknod: HOSTFS.node_ops.mknod,
            rename: HOSTFS.node_ops.rename,
            unlink: HOSTFS.node_ops.unlink,
            rmdir: HOSTFS.node_ops.rmdir,
            readdir: HOSTFS.node_ops.readdir,
            symlink: HOSTFS.node_ops.symlink,
            stat: HOSTFS.node_ops.dir_stat,
        },
        stream: {
            llseek: HOSTFS.stream_ops.llseek,
            getdents: HOSTFS.stream_ops.getdents
        }
    },
    file: {
        node: {
            getattr: HOSTFS.node_ops.getattr,
            setattr: HOSTFS.node_ops.setattr,
            stat: HOSTFS.node_ops.file_stat,
        },
        stream: {
            llseek: HOSTFS.stream_ops.llseek,
            read: HOSTFS.stream_ops.read,
            write: HOSTFS.stream_ops.write,
            open: HOSTFS.stream_ops.open,
            close: HOSTFS.stream_ops.close,
            allocate: HOSTFS.stream_ops.allocate,
            //mmap: HOSTFS.stream_ops.mmap,
            //msync: HOSTFS.stream_ops.msync
        }
    },
};


Module['preRun'] = function () {
    FS.init();
    if (hostfs) {
        for (const vol of hostfs.volumes()) {
            const {id, mount_point, mode} = vol;

            if (mount_point === '@') {
                // Load overlay
                const root_node = HOSTFS.mount({opts: {volid: id}});
                
                for (const name of root_node.node_ops.readdir(root_node)) {

                    if (['dev', 'tmp', 'proc'].includes(name)) {
                        continue;
                    }

                    try {
                        const f = root_node.node_ops.lookup(root_node, name);
                        const n = FS.createNode(FS.root, name, f.mode, 0);

                        n.node_ops = f.node_ops;
                        n.stream_ops = f.stream_ops;
                        n.volid = f.volid;
                        n.tag = f.tag;
                        FS.root.contents[name] = f;

                    }
                    catch (e) {
                        // ignore bad paths
                    }
                }


                continue;
            }

            try {
                FS.createPath('', mount_point, true, true);
            } catch (e) {
                // none
            }
            FS.mount(HOSTFS, {volid: id}, mount_point);
        }
    }

    if ('work_dir' in Module) {
        FS.chdir(Module['work_dir']);
    }

};

