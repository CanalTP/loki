#!/bin/bash

# exit this script with failure on any error
set -e

function show_help() {
    cat << EOF
Usage: ${0##*/}  -o oauth_token [-t tag] [-r -u dockerhub_user -p dockerhub_password]
    -e      [push|pull_request]
    -b      [dev|release] if -e push, or the branch name if -e pull_request
    -f      navitia pull_request fork owner, needed only if -e pull_request
    -o      oauth token for github
    -t      tag images with the given string
    -r      push images to a registry
    -u      username for authentication on dockerhub
    -p      password for authentication on dockerhub

EOF
}

#we want to be able to interupt the build, see: http://veithen.github.io/2014/11/16/sigterm-propagation.html
function run() {
    trap 'kill -TERM $PID' TERM INT
    $@ &
    PID=$!
    wait $PID
    trap - TERM INT
    wait $PID
    return $?
}


while getopts "o:t:b:rp:u:e:f:h" opt; do
    case $opt in
        o) token=$OPTARG
            ;;
        t) tag=$OPTARG
            ;;
        b) branch=$OPTARG
            ;;
        r) push=1
            ;;
        p) password=$OPTARG
            ;;
        u) user=$OPTARG
            ;;
        e) event=$OPTARG
            ;;
        f) fork=$OPTARG
            ;;
        h|\?)
            show_help
            exit 1
            ;;
    esac
done

if [[ -z $token ]] ;
then
     echo "Missing OAuth token for github. Specify one with -o token."
     exit 1
fi

if [[ -z $branch ]];
then
    echo "No branch given. I'll use branch = release  "
    branch="release"
fi

if [[ -z $event ]];
then
    echo "No event given. I'll use event = push  "
    event="push"
fi

if [[ $event == "push" ]]; then
    if [[ $branch == "dev" ]]; then
        workflow="build_navitia_packages_for_dev_multi_distribution.yml"
        archive="navitia-debian10-packages.zip"
        inside_archive="navitia_debian10_packages.zip"
    elif [[ $branch == "release" ]]; then
        workflow="build_navitia_packages_for_release.yml"
        archive="navitia-debian10-packages.zip"
        inside_archive="navitia_debian10_packages.zip"
    else 
        echo """branch must be "dev" or "release" for push events (-e push)"""
        echo "***${branch}***"
        show_help
        exit 1
    fi
    fork="CanalTP"
elif [[ $event == "pull_request" ]]; then
    if [[ -z $branch ]]; then
        echo "branch must be set for pull_request events (-e pull_request -b branch_name)"
        show_help
        exit 1
    fi
    if [[ -z $fork ]]; then
        echo "fork must be set for pull_request events (-e pull_request -f fork)"
        show_help
        exit 1
    fi
    workflow="build_navitia_packages_for_dev_multi_distribution.yml"
    archive="navitia-debian10-packages.zip"
    inside_archive="navitia_debian10_packages.zip"
else
    echo """event must be "push" or "pull_request" """
    echo "***${event}***"
    show_help
    exit 1
fi

if [[ $push -eq 1 ]]; then
    if [ -z $user ];
    then 
        echo """Cannot push to docker registry without a "-u user." """
        show_help
        exit 1
    fi
    if [ -z $password ]; then 
    echo """Cannot push to docker registry without a "-p password." """
        show_help
        exit 1
    fi  
fi

# clone navitia source code with submodules
rm -rf ./tmp/
mkdir -p ./tmp/
run git clone https://x-token-auth:${token}@github.com/${fork}/navitia.git --branch $branch ./tmp/navitia/

# clone loki source code with submodules
run git clone --recursive https://x-token-auth:${token}@github.com/CanalTP/loki.git ./tmp/loki/

# let's dowload the navitia package built on gihub actions
# for that we need the repo core_team_ci_tools
run git clone https://x-token-auth:${token}@github.com/CanalTP/core_team_ci_tools.git  ./tmp/core_team_ci_tools/

# we setup the right python environnement to use core_team_ci_tools
run pip install -r ./tmp/core_team_ci_tools/github_artifacts/requirements.txt --user

# let's download the navitia packages
run python ./tmp/core_team_ci_tools/github_artifacts/github_artifacts.py -o CanalTP -r navitia -t $token -w $workflow -b $branch -a $archive -e $event --output-dir ./tmp/

# let's unzip what we received
run unzip -q ./tmp/${archive} -d ./tmp/

# let's unzip (again) to obtain the packages
run unzip -q ./tmp/${inside_archive} -d ./tmp/

# build the docker for binarisation
run docker build --no-cache -f docker/bina_dockerfile -t mc_navitia/bina  ./tmp/

# build the docker for kraken
run docker build --no-cache -f docker/kraken_dockerfile -t mc_navitia/kraken  ./tmp/

# build the docker for jormun
run docker build --no-cache -f docker/jormun_dockerfile -t mc_navitia/jormun  ./tmp/

# build the docker for server
run docker build --no-cache -f docker/loki_dockerfile -t mc_navitia/loki  ./tmp/


# push image to docker registry if required with -r
# if [[ $push -eq 1 ]]; then
#     docker login -u $user -p $password
#     for component in $components; do
#         docker push navitia/$component:$version
#         # also push tagged image if -t tag was given
#         if [ -n "${tag}" ]; then
#             docker push navitia/$component:$tag
#         fi
#     done
#     docker logout
# fi


# clean up
rm -rf ./tmp/
