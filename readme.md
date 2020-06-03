2020春存储技术基础大作业，实现一个网络学堂映射的用户态文件系统，只支持Linux。

请follow [fuse-rs](https://github.com/zargony/fuse-rs)的安装教程，安装好必要的包之后执行：

```bash
$ mkdit <the folder to mount>
$ cargo run -- <the folder to mount>
```

即可在`<the folder to mount>`中使用网络学堂。有的文件浏览器不能正常显示里面的文件，经测试，`dolphin`是可以的，`thunar`是不行的，你也可以尝试装别的试试看，我并不理解其中的机理。