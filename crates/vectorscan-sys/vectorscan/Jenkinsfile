pipeline {
    agent none
    stages {
        stage("Build") {
            failFast true
            parallel {
                stage("Release/SSE") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-SSE', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=no -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-release-SSE/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-SSE/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Release/AVX2") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-AVX2', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-release-AVX2/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-AVX2/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Release/AVX512") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-AVX512', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-release-AVX512/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-AVX512/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Release/FAT") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-fat', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=yes', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-fat/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/SSE") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-SSE', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=no -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-debug-SSE/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-SSE/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/AVX2") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-AVX2', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-debug-AVX2/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-AVX2/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/AVX512") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-AVX512', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-debug-AVX512/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-AVX512/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/FAT") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-fat', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=yes', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-fat/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Release/ARM") {
                    agent { label "arm" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-arm', buildType: 'Release', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-release-arm/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-arm/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/ARM") {
                    agent { label "arm" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-arm', buildType: 'Debug', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-debug-arm/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-arm/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Release/Power") {
                    agent { label "power" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-release-power', buildType: 'Release', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-release-power/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-release-power/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Debug/Power") {
                    agent { label "power" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-debug-power', buildType: 'Debug', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-debug-power/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-debug-power/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Release/SSE") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-SSE', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=no -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-release-SSE/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-SSE/bin/unit-hyperscan'
                            }
                        }
                    }
                }
                stage("Clang-Release/AVX2") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-AVX2', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-release-AVX2/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-AVX2/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Release/AVX512") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-AVX512', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-release-AVX512/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-AVX512/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Release/FAT") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-fat', buildType: 'Release', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=yes', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-fat/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/SSE") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-SSE', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=no -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-debug-SSE/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-SSE/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/AVX2") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-AVX2', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=no -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-debug-AVX2/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-AVX2/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/AVX512") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-AVX512', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=no', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-debug-AVX512/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-AVX512/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/FAT") {
                    agent { label "x86" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-fat', buildType: 'Debug', cleanBuild: true, cmakeArgs: '-DBUILD_AVX2=yes -DBUILD_AVX512=yes -DFAT_RUNTIME=yes', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-fat/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Release/ARM") {
                    agent { label "arm" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-arm', buildType: 'Release', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-release-arm/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-arm/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/ARM") {
                    agent { label "arm" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-arm', buildType: 'Debug', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-debug-arm/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-arm/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Release/Power") {
                    agent { label "power" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-release-power', buildType: 'Release', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-release-power/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-release-power/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
                stage("Clang-Debug/Power") {
                    agent { label "power" }
                    stages {
                        stage("Git checkout") {
                            steps {
                                checkout([$class: 'GitSCM', branches: [[name: '${sha1}']], extensions: [], userRemoteConfigs: [[refspec: '+refs/pull/${ghprbPullId}/*:refs/remotes/origin/pr/${ghprbPullId}/*', url: 'https://github.com/VectorCamp/vectorscan.git']]])
                            }
                        } 
                        stage("Build") {
                            steps {
                                cmakeBuild buildDir: 'build-clang-debug-power', buildType: 'Debug', cleanBuild: true, cmakeArgs: '', installation: 'InSearchPath', steps: [[envVars: 'CC=clang CXX=clang++', args: '--parallel 4', withCmake: true]]
                            }
                        }
                        stage("Unit Test") {
                            steps {
                                sh 'build-clang-debug-power/bin/unit-internal'
                            }
                        }
                        stage("Test") {
                            steps {
                                sh 'build-clang-debug-power/bin/unit-hyperscan'
                            }
                        }
                    } 
                }
            }
        }
    }
}
