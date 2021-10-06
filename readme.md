[Tsinghua Web Learning](https://learn.tsinghua.edu.cn/) mapped filesystem. Course project for Storage System 2020 Spring.

It only supports Linux. Please follow the installation guide in [fuse-rs](https://github.com/zargony/fuse-rs) to install required packages first.

Assume the folder to mount is `web-learn`.

```
$ mkdir web-learn
$ cargo run -- web-learn
```

Now you can open another terminal and work in the folder `web-learn`.

```
$ cd web-learn
$ mkdir <student id>
<enter password>
$ cd <student id>
# Now play with Tsinghua Web Learning in the terminal. 
```

The filesystem is organized as a tree of `<semester>/<course>/[homework|announcement|file|discussion]>` (in Chinese, `[作业|通知|文件|讨论]`).

You can also use file managers such as [Dolphin](https://apps.kde.org/dolphin/) in the folder.

To terminate and unmount the mapped filesystem:

```
# $ mkdir web-learn
# $ cargo run -- web-learn
<Enter Ctrl + C>
$ fusermount -u web-learn
$ rmdir web-learn
```

For more details, please refer to [report.pdf](report/report.pdf) for a (Chinese) project report。
