# RAPL Library
The RAPL library enables Rust Program to access Linux RAPL energy measurements.

# Known Limitations
Like PAPI, this RAPL uses the MSR kernel module to read module specific registers(MSRs) from user space. To enable the msr module 
interface the admin needs to `chmod 666 /dev/cpu/*/msr`. For kernels older than 3.7, this is all that is required to use this library.

Historically, the Linux MSR driver only relied upon file system checks. This means that anything as root with any capability set could 
read and write to MSRs.

Changes in the mainline Linux kernel since around 3.7 now require an executable to have capability CAP_SYS_RAWIO to open the MSR device file.[1]
Besides loading the MSR kernel module and setting the appropriate file permissions on the msr device file, one must grant the CAP_SYS_RAWIO capability to any user executable that needs access to the MSR driver, using the command below:
```shell
  setcap cap_sys_rawio=ep <user_executable>
```

Note that one needs superuser privileges to grant the RAWIO capability to an executable, and that the executable cannot be located on a shared network file system partition.

The dynamic linker on most operating systems will remove variables that control dynamic linking from the environment of executables with extended rights, such as setuid executables or executables with raised capabilities. One such variable is LD_LIBRARY_PATH. Therefore, executables that have the RAWIO capability can only load shared libraries from default system directories. One can work around this restriction by either installing the shared libraries in system directories, linking statically against those libraries, or using the -rpath linker option to specify the full path to the shared libraries during the linking step.

# References
[1] https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=c903f0456bc69176912dee6dd25c6a66ee1aed00
