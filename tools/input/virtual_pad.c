/* Authored Linux-only test device. Sends controller events, never keyboard/mouse. */
#include <linux/uinput.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
static int pads[2]={-1,-1};
static void require(int result) {if(result<0){perror("uinput");exit(1);}}
static void close_pad(int id) {if(pads[id]>=0){ioctl(pads[id],UI_DEV_DESTROY);close(pads[id]);pads[id]=-1;}}
static void create_pad(int id) {
    if(pads[id]>=0)return;
    int fd=open("/dev/uinput",O_WRONLY|O_NONBLOCK);require(fd);
    require(ioctl(fd,UI_SET_EVBIT,EV_KEY));require(ioctl(fd,UI_SET_EVBIT,EV_ABS));
    int buttons[]={BTN_SOUTH,BTN_EAST,BTN_NORTH,BTN_WEST,BTN_TL,BTN_TR,BTN_SELECT,BTN_START,BTN_MODE,BTN_THUMBL,BTN_THUMBR};
    for(unsigned i=0;i<sizeof(buttons)/sizeof(buttons[0]);++i)require(ioctl(fd,UI_SET_KEYBIT,buttons[i]));
    int axes[]={ABS_X,ABS_Y,ABS_Z,ABS_RZ,ABS_HAT0X,ABS_HAT0Y,ABS_HAT2X,ABS_HAT2Y};
    for(unsigned i=0;i<sizeof(axes)/sizeof(axes[0]);++i){
        require(ioctl(fd,UI_SET_ABSBIT,axes[i]));
        struct uinput_abs_setup a={0};a.code=axes[i];
        a.absinfo.minimum=axes[i]==ABS_X||axes[i]==ABS_Y?-32768:(axes[i]==ABS_HAT0X||axes[i]==ABS_HAT0Y)?-1:0;
        a.absinfo.maximum=axes[i]==ABS_X||axes[i]==ABS_Y?32767:(axes[i]==ABS_HAT0X||axes[i]==ABS_HAT0Y)?1:255;
        require(ioctl(fd,UI_ABS_SETUP,&a));
    }
    struct uinput_setup setup={0};setup.id.bustype=BUS_USB;setup.id.vendor=0xfffe;setup.id.product=0x100+id;setup.id.version=1;
    snprintf(setup.name,sizeof(setup.name),"RoadRash Virtual Test Pad %d",id);
    require(ioctl(fd,UI_DEV_SETUP,&setup));require(ioctl(fd,UI_DEV_CREATE));pads[id]=fd;
}
static void emit(int id,int type,int code,int value) {
    if(pads[id]<0){fprintf(stderr,"pad absent\n");exit(2);}
    struct input_event event={0};event.type=type;event.code=code;event.value=value;
    require((int)write(pads[id],&event,sizeof(event)));
    memset(&event,0,sizeof(event));event.type=EV_SYN;event.code=SYN_REPORT;require((int)write(pads[id],&event,sizeof(event)));
}
int main(void) {
    char line[128],operation[24];int id,value;
    setvbuf(stdout,NULL,_IOLBF,0);
    while(fgets(line,sizeof(line),stdin)) {
        int count=sscanf(line,"%23s %d %d",operation,&id,&value);
        if(count<2||id<0||id>1)return 2;
        if(!strcmp(operation,"create"))create_pad(id);
        else if(!strcmp(operation,"destroy"))close_pad(id);
        else if(count!=3)return 2;
        else if(!strcmp(operation,"axis"))emit(id,EV_ABS,ABS_X,value);
        else if(!strcmp(operation,"south"))emit(id,EV_KEY,BTN_SOUTH,value);
        else if(!strcmp(operation,"east"))emit(id,EV_KEY,BTN_EAST,value);
        else if(!strcmp(operation,"start"))emit(id,EV_KEY,BTN_START,value);
        else if(!strcmp(operation,"trigger"))emit(id,EV_ABS,ABS_HAT2X,value);
        else return 2;
        printf("ok %s %d\n",operation,id);
    }
    close_pad(0);close_pad(1);return 0;
}
