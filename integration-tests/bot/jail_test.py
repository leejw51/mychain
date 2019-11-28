#!/usr/bin/python3
import docker
import json
import requests
import datetime
import time
from chainrpc import RPC
class Program :
    def __init__(self) :
        self.rpc = RPC()

        # wallet a
        self.node0_address = ""
        self.node0_mnemonics= ""

        # wallet b
        self.node1_address = ""
        self.node1_mnemonics=""
        self.server="http://localhost:26657"
        self.client_rpc= "http://localhost:9981"
        self.headers = {
            'Content-Type': 'application/json',
        }

    def get_containers(self) :
        client = docker.from_env()
        containers= client.containers.list()
        ret= {}
        for container in containers:
            id = container
            #ret[id.name]= id.id
            ret[id.name]= container
        return ret
        

    #show_containers()
    # tendermint rpc



    def get_staking_state(self,name, passphrase, addr):
        q = {
            "method": "staking_state",
            "jsonrpc": "2.0",
            "params": [{
                "name": name,
                "passphrase": passphrase
            }, addr],
            "id": "staking_state"
        }
        data = json.dumps(q)
        response = requests.post(self.client_rpc, headers=self.headers, data=data)
        return response.json()["result"]




    def create_staking_address(self,name, passphrase):
        q = {
            "method": "wallet_createStakingAddress",
            "jsonrpc": "2.0",
            "params": [{
                "name": name,
                "passphrase": passphrase
            }],
            "id": "wallet_createStakingAddress"
        }
        data = json.dumps(q)
        response = requests.post(self.client_rpc, headers=self.headers, data=data)


    def restore_wallet(self,name, passphrase, mnemonics):
        q = {
            "method": "wallet_restore",
            "jsonrpc": "2.0",
            "params": [{
                "name": name,
                "passphrase": passphrase
            }, mnemonics],
            "id": "wallet_restore_hd"
        }
        data = json.dumps(q)
        response = requests.post(self.client_rpc, headers=self.headers, data=data)
        print("restore wallet {}".format(name), response.json())

    def restore_wallets(self):
        print("restore wallets")
        self.rpc.wallet.restore(self.node0_mnemonics, "a")
        self.rpc.wallet.restore(self.node0_mnemonics, "b")
        self.restore_wallet(
            "a", "1",
            self.node0_mnemonics
        )
        self.restore_wallet(
            "b", "1",
            self.node1_mnemonics
        )


    def create_addresses(self):
        self.create_staking_address("a", "1")
        self.create_staking_address("a", "1")
        self.create_staking_address("b", "1")
        self.create_staking_address("b", "1")
        

    def unjail(self,name, passphrase, address):
        q = {
            "method": "staking_unjail",
            "jsonrpc": "2.0",
            "params": [{
                "name": name,
                "passphrase": passphrase
            }, address],
            "id": "staking_unjail"
        }
        data = json.dumps(q)
        response = requests.post(self.client_rpc, headers=self.headers, data=data)
        print(response.json())
        return response.json()


    def check_validators(self) :
        try: 
            x= requests.get('{}/validators'.format(self.server))
            data =len(x.json()["result"]["validators"])
            return data
        except requests.ConnectionError:
            return 0
        except:
            assert False

    def wait_for_ready(self,count) :
        initial_time=time.time() # in seconds
        MAX_TIME = 3600
        while True:
            current_time= time.time()
            elasped_time= current_time - initial_time
            remain_time = MAX_TIME - elasped_time
            validators=self.check_validators()
            if remain_time< 0 :
                assert False
            print("{0}  remain time={1:.2f}  current validators={2}  waiting for validators={3}".format(datetime.datetime.now(), remain_time, validators, count))
            if count== validators :
                print("validators ready")
                break
            time.sleep(10)


    def test_jailing(self) :
        print("test jailing")
        self.wait_for_ready(2)
        containers=self.get_containers()
        print(containers)
        if "jail_chain1_1" in containers :
            assert True
        else :
            assert False
        print("wait for jailing")
        time.sleep(10)
        jailthis = containers["jail_chain1_1"]
        print("jail = " , jailthis)
        jailthis.kill()
        self.wait_for_ready(1)
        #jailed
        containers=self.get_containers()
        print(containers)
        if "jail_chain1_1" in containers :
            assert False
        else :
            assert True 
        print("jail test success")


    def test_unjailing(self) :
        initial_time=time.time() # in seconds
        print("test unjailing")
        self.wait_for_ready(1)

        count=2
        MAX_TIME = 3600  
        while True:
            current_time= time.time()
            elasped_time= current_time - initial_time
            remain_time = MAX_TIME - elasped_time
            validators=self.check_validators()
            if remain_time< 0 :
                assert False
            self.unjail("b","1", self.node1_address)
            state= self.get_staking_state("b","1", self.node1_address)
            punishment=state["punishment"] 
            print("{0}  remain time={1:.2f}  punishment {2}".format(datetime.datetime.now(), remain_time, punishment))
            if punishment== None :
                print("unjailed!!")
                break
            else :
                print("still jailed")
            time.sleep(10)
        print("unjail test success")

    ############################################################################3
    def main (self) :
        #self.test_jailing()
        self.restore_wallets()
        #self.create_addresses()
        #self.test_unjailing()


    def read_info(self):
        print("read data")
        with open('nodes_info.json') as json_file:
            data = json.load(json_file)
        print(json.dumps(data,indent=4))
        self.node0_address= data["nodes"][0]["staking"][0]
        self.node1_address= data["nodes"][1]["staking"][0]

        self.node0_mnemonics=data["nodes"][0]["mnemonic"]
        self.node1_mnemonics=data["nodes"][1]["mnemonic"]
        
    def display_info(self):
        print("node0 staking= {}".format(self.node0_address))
        print("node1 staking= {}".format(self.node1_address))
        print("node0 mnemonics= {}".format(self.node0_mnemonics))
        print("node1 mnemonics= {}".format(self.node1_mnemonics))


p = Program()
p.read_info()
p.display_info()
p.main()