import os
from dotenv import load_dotenv
load_dotenv("./.env")

from subprocess import *
from pwn import *
import threading
import re

TIMEOUT=5000

def runNode(command, nodeNum):
    print("Node "+str(nodeNum) + " started..")
    p = process(command)
    batchCount = 1
    
    # batch = {"requests" : dict(), "eventDetectedTime" : dict(), "signedTime" : dict(), "callToDestCompletedTime": dict(), "fundsReleased" : dict()}
    while True:
        # batch = []
        output = []
        # try :
            # batchDetected = p.recvline(timeout=1200).decode().strip().split()[-1]
            # batchSize = p.recvline(timeout=1200).decode().strip().split()[-1]
            # signedTime = p.recvline(timeout=1200).decode().strip().split()[-1]
            # callToDest = p.recvline(timeout=1200).decode().strip().split()[-1]
            # callDelay = p.recvline(timeout=1200).decode().strip().split()[-1]
            # temp = p.recvline(timeout=1200).decode().strip()
            # print(temp)
            
            # batch.append("Node "+str(nodeNum)+"\n")
            # batch.append("Batch "+str(batchCount)+"\n")
            # batch.append("Batch detected at "+batchDetected+"\n")
            # batch.append("Sign completed at "+signedTime+"\n")
            # batch.append("Call to destination completed at "+callToDest+"\n")
            # batch.append("Call completion delay "+callDelay+"\n")


        output.append("Batch_Number = "+str(batchCount))

        tmp = p.recvuntil(b"BATCH CALL END", timeout=1200).decode().split("\n")[:-1]
        print(tmp)
        ListOfeventDelaysString = tmp[:-5]
        timeToDetectLastEvent = int(ListOfeventDelaysString[-1].split()[-1])
        output.append("Event_Detect_Delay = "+str(timeToDetectLastEvent))

        times = tmp[-5:]
        output.extend(times)

        Batch_Number = re.findall(r"[-+]?(?:\d*\.*\d+)",output[0])[0]
        Event_Detect_Delay= float(re.findall(r"[-+]?(?:\d*\.*\d+)",output[1])[0])
        BATCHED= int(re.findall(r"[-+]?(?:\d*\.*\d+)",output[2])[0])
        BATCH_SIZE= int(re.findall(r"[-+]?(?:\d*\.*\d+)",output[3])[0])
        BATCH_SIGNED = int(re.findall(r"[-+]?(?:\d*\.*\d+)",output[4])[0])
        CALL_TO_DESTINATION_COMPLETED= int(re.findall(r"[-+]?(?:\d*\.*\d+)",output[5])[0])
        CALL_DELAY= int(re.findall(r"[-+]?(?:\d*\.*\d+)",output[6])[0])
        
        results_summary = []
        results_summary.append("Batch number = "+str(Batch_Number)+"\n")
        results_summary.append("Batch size = "+str(BATCH_SIZE)+"\n")
        results_summary.append("Time to detect last event of the batch = "+str(Event_Detect_Delay*1000)+"\n")
        results_summary.append("Time to sign batch = "+str(BATCH_SIGNED-BATCHED)+"\n")
        # results_summary.append("Destination call completion (from after signing) = "+str(CALL_TO_DESTINATION_COMPLETED-BATCH_SIGNED-(nodeNum*TIMEOUT))+"\n")
        results_summary.append("Time to Destination call completion = "+str(CALL_DELAY)+"\n")

        batchCount+=1
        # print(output)

        f = open("./test_results/test4_9_20/node"+str(nodeNum)+".txt", "a") #+str(nodeNum)+
        f.writelines(results_summary)
        f.close()


def returnNodes(num):
    commands = []
    timeout = 0
    for i in range(1, num+1):
        cmd = []
        cmd.append("./target/debug/hyperloop_relaynode")
        cmd.append(os.environ["NODE"+str(i)+"_PRIVATE_KEY"])
        cmd.append(os.environ["NODE"+str(i)+"_PUBLIC_KEY"])
        cmd.append(str(timeout))
        timeout += TIMEOUT #in ms
        
        commands.append(cmd)
    return commands

if __name__ =="__main__":
    commands = returnNodes(9)

    t1 = threading.Thread(target=runNode, args=(commands[0], 1, ))
    t2 = threading.Thread(target=runNode, args=(commands[1], 2, ))
    t3 = threading.Thread(target=runNode, args=(commands[2], 3, ))
    t4 = threading.Thread(target=runNode, args=(commands[3], 4, ))
    t5 = threading.Thread(target=runNode, args=(commands[4], 5, ))
    t6 = threading.Thread(target=runNode, args=(commands[5], 6, ))
    t7 = threading.Thread(target=runNode, args=(commands[6], 7, ))
    t8 = threading.Thread(target=runNode, args=(commands[7], 8, ))
    t9 = threading.Thread(target=runNode, args=(commands[8], 9, ))
    # t10 = threading.Thread(target=runNode, args=(commands[9], 10, ))
    # t11 = threading.Thread(target=runNode, args=(commands[10], 11, ))
    # t12 = threading.Thread(target=runNode, args=(commands[11], 12, ))
    # t13 = threading.Thread(target=runNode, args=(commands[12], 13, ))
    # t14 = threading.Thread(target=runNode, args=(commands[13], 14, ))
    # t15 = threading.Thread(target=runNode, args=(commands[14], 15, ))
    # t16 = threading.Thread(target=runNode, args=(commands[15], 16, ))
    # t17 = threading.Thread(target=runNode, args=(commands[16], 17, ))
    # t18 = threading.Thread(target=runNode, args=(commands[17], 18, ))
    # t19 = threading.Thread(target=runNode, args=(commands[18], 19, ))
    # t20 = threading.Thread(target=runNode, args=(commands[19], 20, ))

    t1.start()
    t2.start()
    t3.start()
    t4.start()
    t5.start()
    t6.start()
    t7.start()
    t8.start()
    t9.start()
    # t10.start()
    # t11.start()
    # t12.start()
    # t13.start()
    # t14.start()
    # t15.start()
    # t16.start()
    # t17.start()
    # t18.start()
    # t19.start()
    # t20.start()

    t1.join()
    t2.join()
    t3.join()
    t4.join()
    t5.join()
    t6.join()
    t7.join()
    t8.join()
    t9.join()
    # t10.join()
    # t11.join()
    # t12.join()
    # t13.join()
    # t14.join()
    # t15.join()
    # t16.join()
    # t17.join()
    # t18.join()
    # t19.join()
    # t20.join()

# {
#     req : {
#         req1 : actionId,
#         req2 : actionId,
#     }
#     eventDetected : {
#         node1 : ,
#         node2 : ,
#     },
#     signedTime : {
#         node1 : ,
#         node2 : ,
#     },
#     releasedTime : {
#         req1 : ,
#         req2 : ,
#     }

# }